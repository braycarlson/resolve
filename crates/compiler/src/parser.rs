use crate::ast::*;
use crate::error::{Diagnostic, ParseError, Severity};
use crate::lexer::{Lexer, Token};


const PARSE_ITERATIONS_MAX: u32 = 1_000_000;
const NESTING_DEPTH_MAX: u32 = 128;

pub struct ParseOutput {
    pub nodes: Vec<AstNode>,
    pub diagnostics: Vec<Diagnostic>,
}

fn convert_filter(filters: &[crate::lexer::Filter<'_>]) -> Vec<Filter> {
    filters.iter().map(|filter| Filter {
        name: filter.name.to_string(),
        arguments: filter.arguments.iter().map(|argument| argument.to_string()).collect(),
    }).collect()
}

fn split_path(input: &str) -> (&str, &str) {
    let bytes = input.as_bytes();
    let length = bytes.len();

    if length == 0 {
        return ("", "");
    }

    let mut position: usize = 0;

    if bytes[0] == b'\'' || bytes[0] == b'"' {
        let quote = bytes[0];
        position = 1;

        while position < length && bytes[position] != quote {
            position += 1;
        }

        if position < length {
            position += 1;
        }
    } else {
        while position < length && bytes[position] != b' ' && bytes[position] != b'\t' {
            position += 1;
        }
    }

    (&input[..position], &input[position..])
}

fn parse_bindings(input: &str) -> (Vec<Binding>, bool) {
    let bytes = input.as_bytes();
    let length = bytes.len();
    let mut bindings = Vec::new();
    let mut only = false;
    let mut position: usize = 0;
    let mut iterations: u32 = 0;

    while position < length {
        iterations += 1;

        assert!(
            iterations <= PARSE_ITERATIONS_MAX,
            "parse_bindings exceeded maximum iterations",
        );

        while position < length && (bytes[position] == b' ' || bytes[position] == b'\t') {
            position += 1;
        }

        if position >= length {
            break;
        }

        if position + 4 <= length
            && &input[position..position + 4] == "only"
            && (position + 4 == length
                || bytes[position + 4] == b' '
                || bytes[position + 4] == b'\t')
        {
            only = true;
            break;
        }

        let start = position;

        while position < length
            && bytes[position] != b'='
            && bytes[position] != b' '
            && bytes[position] != b'\t'
        {
            position += 1;
        }

        if position >= length || bytes[position] != b'=' {
            while position < length && bytes[position] != b' ' && bytes[position] != b'\t' {
                position += 1;
            }

            continue;
        }

        let name = input[start..position].to_string();
        position += 1;

        if position >= length {
            break;
        }

        let start = position;

        if bytes[position] == b'\'' || bytes[position] == b'"' {
            let quote = bytes[position];
            position += 1;

            while position < length && bytes[position] != quote {
                position += 1;
            }

            if position < length {
                position += 1;
            }
        } else {
            while position < length && bytes[position] != b' ' && bytes[position] != b'\t' {
                if bytes[position] == b'\'' || bytes[position] == b'"' {
                    let quote = bytes[position];
                    position += 1;

                    while position < length && bytes[position] != quote {
                        position += 1;
                    }

                    if position < length {
                        position += 1;
                    }
                } else {
                    position += 1;
                }
            }
        }

        let value = input[start..position].to_string();
        bindings.push(Binding { name, value });
    }

    (bindings, only)
}

pub struct Parser<'a> {
    tokens: Vec<Token<'a>>,
    position: u32,
    depth: u32,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token<'a>>) -> Self {
        debug_assert!(
            tokens.last() == Some(&Token::Eof),
            "token stream must end with Eof",
        );

        assert!(
            tokens.len() <= u32::MAX as usize,
            "token count exceeds u32 maximum",
        );

        Self {
            tokens,
            position: 0,
            depth: 0,
            diagnostics: Vec::new(),
        }
    }

    fn warn(&mut self, message: String, position: u32) {
        self.diagnostics.push(Diagnostic {
            severity: Severity::Warning,
            message,
            position,
        });
    }

    fn error(&mut self, message: String, position: u32) {
        self.diagnostics.push(Diagnostic {
            severity: Severity::Error,
            message,
            position,
        });
    }

    fn enter(&mut self) -> Result<(), ParseError> {
        self.depth += 1;

        if self.depth > NESTING_DEPTH_MAX {
            return Err(ParseError::Generic(format!(
                "Template nesting depth exceeds maximum of {}",
                NESTING_DEPTH_MAX,
            )));
        }

        Ok(())
    }

    fn leave(&mut self) {
        assert!(
            self.depth > 0,
            "nesting depth underflow",
        );

        self.depth -= 1;
    }

    pub fn parse(&mut self) -> Result<Vec<AstNode>, ParseError> {
        let mut nodes = Vec::with_capacity(self.tokens.len() / 2);
        let mut iterations: u32 = 0;

        while !self.done() {
            iterations += 1;

            assert!(
                iterations <= PARSE_ITERATIONS_MAX,
                "parse exceeded maximum iterations",
            );

            if let Some(node) = self.parse_node()? {
                nodes.push(node);
            }
        }

        Ok(nodes)
    }

    fn done(&self) -> bool {
        let position = self.position as usize;

        position >= self.tokens.len()
            || matches!(self.tokens[position], Token::Eof)
    }

    fn current(&self) -> &Token<'a> {
        &self.tokens[self.position as usize]
    }

    fn advance(&mut self) {
        if !self.done() {
            self.position += 1;
        }
    }

    fn parse_until(&mut self, end_tag: &str) -> Result<Vec<AstNode>, ParseError> {
        self.enter()?;

        let mut body = Vec::new();
        let mut iterations: u32 = 0;
        let start = self.position;

        loop {
            iterations += 1;

            assert!(
                iterations <= PARSE_ITERATIONS_MAX,
                "parse_until exceeded maximum iterations for tag '{end_tag}'",
            );

            if self.done() {
                self.warn(
                    format!(
                        "Unclosed '{}' tag (opened at token {})",
                        end_tag,
                        start,
                    ),
                    start,
                );

                break;
            }

            let mark = self.position;

            if let Token::BlockEnd { tag, .. } = self.current() {
                if *tag == end_tag {
                    self.advance();
                    break;
                }
            }

            match self.parse_node() {
                Ok(Some(node)) => body.push(node),
                Ok(None) => {
                    if self.position == mark {
                        self.warn(
                            format!(
                                "Skipped unparseable token inside '{}'",
                                end_tag,
                            ),
                            mark,
                        );

                        self.advance();
                    }
                }
                Err(_) => {
                    if self.position == mark {
                        self.error(
                            format!(
                                "Error parsing child of '{}', skipping token",
                                end_tag,
                            ),
                            mark,
                        );

                        self.advance();
                    }
                }
            }
        }

        self.leave();

        Ok(body)
    }

    fn parse_node(&mut self) -> Result<Option<AstNode>, ParseError> {
        if self.done() {
            return Ok(None);
        }

        match self.current().clone() {
            Token::Text(content) => {
                self.advance();

                Ok(Some(AstNode::Text(TextNode {
                    content: content.to_string(),
                })))
            }

            Token::Variable { expression, filters, raw } => {
                self.advance();

                let filters = convert_filter(&filters);

                if let Some(node) =
                    self.try_json_script(expression, &filters, raw)
                {
                    Ok(Some(node))
                } else {
                    Ok(Some(AstNode::Variable(Box::new(VariableNode {
                        raw: raw.to_string(),
                        expression: expression.to_string(),
                        filters,
                    }))))
                }
            }

            Token::BlockStart { raw, tag, content } => {
                self.advance();
                self.parse_tag(tag, content, raw)
            }

            Token::BlockEnd { raw, .. } => {
                self.advance();
                Ok(Some(AstNode::Text(TextNode {
                    content: raw.to_string(),
                })))
            }

            Token::Comment(content) => {
                self.advance();
                Ok(Some(AstNode::Text(TextNode {
                    content: format!("{{# {} #}}", content),
                })))
            }

            Token::Eof => Ok(None),
        }
    }

    fn parse_tag(
        &mut self,
        tag: &str,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        match tag {
            "block" => self.parse_block(content, raw),
            "extends" => self.parse_extends(content, raw),
            "include" => self.parse_include(content, raw),
            "if" => self.parse_if(content, raw),
            "for" => self.parse_for(content, raw),
            "with" => self.parse_with(content, raw),
            "load" => self.parse_load(content, raw),
            "csrf_token" => Ok(Some(AstNode::Csrftoken(CsrftokenNode {
                raw: raw.to_string(),
            }))),
            "comment" => self.parse_comment_block(raw),
            "autoescape" => self.parse_autoescape(content, raw),
            "blocktranslate" => self.parse_blocktranslate(content, raw),
            "translate" => self.parse_translate(content, raw),
            "trans" => self.parse_trans(content, raw),
            "language" => self.parse_language(content, raw),
            "verbatim" => self.parse_verbatim(raw),
            "templatetag" => self.parse_templatetag(content, raw),
            "ifchanged" => self.parse_ifchanged(content, raw),
            "filter" => self.parse_filter(content, raw),
            "now" => self.parse_now(content, raw),
            "cycle" => self.parse_cycle(content, raw),
            "firstof" => self.parse_firstof(content, raw),
            "widthratio" => self.parse_widthratio(content, raw),
            "url" => self.parse_url(content, raw),
            "static" => self.parse_static_capture(content, raw),
            "cache" => self.parse_cache(content, raw),
            "localize" => self.parse_localize(content, raw),
            "localtime" => self.parse_localtime(content, raw),
            "spaceless" => self.parse_spaceless(raw),
            "timezone" => self.parse_timezone(content, raw),
            "utc" => self.parse_utc(raw),
            "debug" => Ok(Some(AstNode::Debug(DebugNode {
                raw: raw.to_string(),
            }))),
            "get_static_prefix" => self.parse_get_static_prefix(content, raw),
            "get_media_prefix" => self.parse_get_media_prefix(content, raw),
            "resetcycle" => self.parse_resetcycle(content, raw),
            "lorem" => self.parse_lorem(content, raw),
            "querystring" => self.parse_querystring(content, raw),
            "plural" => self.parse_plural(content, raw),
            "regroup" => self.parse_regroup(content, raw),
            _ => {
                Ok(Some(AstNode::Text(TextNode {
                    content: raw.to_string(),
                })))
            }
        }
    }

    fn parse_block(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        self.enter()?;

        let name = content.trim_start_matches("block").trim();
        let mut body = Vec::new();
        let mut has_super = false;
        let mut iterations: u32 = 0;
        let start = self.position;

        loop {
            iterations += 1;
            assert!(iterations <= PARSE_ITERATIONS_MAX, "parse_block stuck");

            if self.done() {
                self.warn(
                    format!(
                        "Unclosed block '{}' (opened at token {})",
                        name,
                        start,
                    ),
                    start,
                );
                break;
            }

            let mark = self.position;

            if let Token::BlockEnd { tag, .. } = self.current() {
                if *tag == "block" {
                    self.advance();
                    break;
                }
            }

            if let Some(node) = self.parse_node()? {
                if matches!(
                    &node,
                    AstNode::Variable(variable) if variable.expression.trim() == "block.super"
                ) {
                    has_super = true;
                }
                body.push(node);
            }

            if self.position == mark {
                self.advance();
            }
        }

        self.leave();

        Ok(Some(AstNode::Block(Box::new(BlockNode {
            raw: raw.to_string(),
            name: name.to_string(),
            content: body,
            has_super_reference: has_super,
        }))))
    }

    fn parse_extends(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let path = content
            .trim_start_matches("extends")
            .trim()
            .trim_matches('\'')
            .trim_matches('"');

        if path.is_empty() {
            self.error(
                "{% extends %} tag has empty path".to_string(),
                self.position.saturating_sub(1),
            );
        }

        Ok(Some(AstNode::Extends(ExtendsNode {
            raw: raw.to_string(),
            parent_path: path.to_string(),
        })))
    }

    fn parse_include(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let remainder = content.trim_start_matches("include").trim();
        let (segment, remainder) = split_path(remainder);

        let path = segment
            .trim_matches('\'')
            .trim_matches('"');

        if path.is_empty() {
            self.error(
                "{% include %} tag has empty path".to_string(),
                self.position.saturating_sub(1),
            );
        }

        let mut with_variables = Vec::new();
        let mut only = false;

        let remainder = remainder.trim();

        if remainder == "only" {
            only = true;
        } else if remainder.starts_with("with")
            && (remainder.len() == 4
                || remainder.as_bytes().get(4).is_some_and(
                    |&byte| byte == b' ' || byte == b'\t',
                ))
        {
            let remainder = remainder[4..].trim_start();

            if remainder == "only" {
                only = true;
            } else if !remainder.is_empty() {
                let (bindings, is_only) = parse_bindings(remainder);
                with_variables = bindings;
                only = is_only;
            }
        }

        Ok(Some(AstNode::Include(Box::new(IncludeNode {
            raw: raw.to_string(),
            path: path.to_string(),
            with_variables,
            only,
        }))))
    }

    fn parse_if(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        self.enter()?;

        let condition = content.trim_start_matches("if").trim();
        let mut true_branch = Vec::new();
        let mut elif_branches: Vec<ElifBranch> = Vec::new();
        let mut else_branch: Option<Vec<AstNode>> = None;

        #[derive(PartialEq)]
        enum IfPhase { True, Elif, Else }
        let mut phase = IfPhase::True;
        let mut iterations: u32 = 0;
        let start = self.position;

        loop {
            iterations += 1;
            assert!(iterations <= PARSE_ITERATIONS_MAX, "parse_if stuck");

            if self.done() {
                self.warn(
                    format!(
                        "Unclosed if tag (opened at token {})",
                        start,
                    ),
                    start,
                );
                break;
            }

            let mark = self.position;
            let token = self.current().clone();

            if let Token::BlockEnd { tag, .. } = token {
                if tag == "if" {
                    self.advance();
                    break;
                }
            }

            if let Token::BlockStart {
                tag,
                content: body,
                ..
            } = token
            {
                match tag {
                    "elif" => {
                        self.advance();

                        let condition =
                            body.trim_start_matches("elif").trim().to_string();

                        elif_branches.push(ElifBranch {
                            condition,
                            body: Vec::new(),
                        });

                        phase = IfPhase::Elif;
                        continue;
                    }
                    "else" if phase != IfPhase::Else => {
                        self.advance();
                        else_branch = Some(Vec::new());
                        phase = IfPhase::Else;
                        continue;
                    }
                    _ => {}
                }
            }

            match self.parse_node()? {
                Some(node) => match phase {
                    IfPhase::True => true_branch.push(node),

                    IfPhase::Elif => {
                        if let Some(last) = elif_branches.last_mut() {
                            last.body.push(node);
                        }
                    }

                    IfPhase::Else => {
                        if let Some(ref mut branch) = else_branch {
                            branch.push(node);
                        }
                    }
                },
                None => {
                    if self.position == mark {
                        self.advance();
                    }
                }
            }

            if self.position == mark {
                self.advance();
            }
        }

        self.leave();

        Ok(Some(AstNode::If(Box::new(IfNode {
            raw: raw.to_string(),
            condition: condition.to_string(),
            true_branch,
            elif_branches,
            else_branch,
        }))))
    }

    fn parse_for(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        self.enter()?;

        let inner = content.trim_start_matches("for").trim();
        let parts: Vec<&str> = inner.split(" in ").collect();
        let variable = parts.first().unwrap_or(&"").to_string();
        let iterable = parts.get(1).unwrap_or(&"").to_string();

        if variable.is_empty() || iterable.is_empty() {
            self.warn(
                format!("Malformed for tag: '{}'", content),
                self.position.saturating_sub(1),
            );
        }

        let mut body = Vec::new();
        let mut empty_branch = None;
        let mut in_empty = false;
        let mut iterations: u32 = 0;
        let start = self.position;

        loop {
            iterations += 1;
            assert!(iterations <= PARSE_ITERATIONS_MAX, "parse_for stuck");

            if self.done() {
                self.warn(
                    format!(
                        "Unclosed for tag (opened at token {})",
                        start,
                    ),
                    start,
                );

                break;
            }

            let mark = self.position;

            if let Token::BlockEnd { tag, .. } = self.current() {
                if *tag == "for" {
                    self.advance();
                    break;
                }
            }

            if let Token::BlockStart { tag, .. } = self.current() {
                if *tag == "empty" && !in_empty {
                    self.advance();
                    in_empty = true;
                    empty_branch = Some(Vec::new());
                    continue;
                }
            }

            match self.parse_node()? {
                Some(node) => {
                    if in_empty {
                        if let Some(ref mut branch) = empty_branch {
                            branch.push(node);
                        }
                    } else {
                        body.push(node);
                    }
                }
                None => {
                    if self.position == mark {
                        self.advance();
                    }
                }
            }

            if self.position == mark {
                self.advance();
            }
        }

        self.leave();

        Ok(Some(AstNode::For(Box::new(ForNode {
            raw: raw.to_string(),
            variable,
            iterable,
            body,
            empty_branch,
        }))))
    }

    fn parse_with(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let inner = content.trim_start_matches("with").trim();
        let (bindings, _) = parse_bindings(inner);
        let body = self.parse_until("with")?;

        Ok(Some(AstNode::With(Box::new(WithNode {
            raw: raw.to_string(),
            bindings,
            body,
        }))))
    }

    fn parse_load(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let libraries = content
            .trim_start_matches("load")
            .split_whitespace()
            .map(|string| string.to_string())
            .collect();

        Ok(Some(AstNode::Load(LoadNode {
            raw: raw.to_string(),
            libraries,
        })))
    }

    fn parse_static(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let inner = content.trim_start_matches("static").trim();
        let path = inner.trim_matches('\'').trim_matches('"');

        Ok(Some(AstNode::Static(StaticNode {
            raw: raw.to_string(),
            path: path.to_string(),
        })))
    }

    fn parse_comment_block(
        &mut self,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        self.enter()?;

        let mut content = String::new();

        loop {
            if self.done() {
                break;
            }

            if let Token::BlockEnd { tag, .. } = self.current() {
                if *tag == "comment" {
                    self.advance();
                    break;
                }
            }

            if let Some(AstNode::Text(text)) = self.parse_node()? {
                content.push_str(&text.content);
            }
        }

        self.leave();

        Ok(Some(AstNode::CommentBlock(CommentBlockNode {
            raw: raw.to_string(),
            content,
        })))
    }

    fn parse_autoescape(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let value = content.trim_start_matches("autoescape").trim();
        let body = self.parse_until("autoescape")?;

        Ok(Some(AstNode::Autoescape(Box::new(AutoescapeNode {
            raw: raw.to_string(),
            value: value.to_string(),
            body,
        }))))
    }

    fn parse_blocktranslate(
        &mut self,
        _content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let body = self.parse_until("blocktranslate")?;

        Ok(Some(AstNode::Blocktranslate(Box::new(BlocktranslateNode {
            raw: raw.to_string(),
            body,
            modifiers: Vec::new(),
        }))))
    }

    fn parse_trans(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let message = content
            .trim_start_matches("trans")
            .trim()
            .trim_matches('\'')
            .trim_matches('"');

        Ok(Some(AstNode::Trans(TransNode {
            raw: raw.to_string(),
            message: message.to_string(),
        })))
    }

    fn parse_language(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let language = content
            .trim_start_matches("language")
            .trim()
            .trim_matches('\'')
            .trim_matches('"');

        let body = self.parse_until("language")?;

        Ok(Some(AstNode::Language(Box::new(LanguageNode {
            raw: raw.to_string(),
            language: language.to_string(),
            body,
        }))))
    }

    fn parse_verbatim(
        &mut self,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let mut content = String::new();

        if let Token::Text(text) = self.current().clone() {
            content = text.to_string();
            self.advance();
        }

        Ok(Some(AstNode::Verbatim(VerbatimNode {
            raw: raw.to_string(),
            content,
        })))
    }

    fn parse_templatetag(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let format = content.trim_start_matches("templatetag").trim();
        let format = format.trim_matches('\'').trim_matches('"');

        Ok(Some(AstNode::TemplateTag(TemplateTagNode {
            raw: raw.to_string(),
            format: format.to_string(),
        })))
    }

    fn parse_ifchanged(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        self.enter()?;

        let condition = content.trim_start_matches("ifchanged").trim();
        let mut true_branch = Vec::new();
        let mut else_branch: Option<Vec<AstNode>> = None;
        let mut in_else = false;
        let mut iterations: u32 = 0;
        let start = self.position;

        loop {
            iterations += 1;
            assert!(iterations <= PARSE_ITERATIONS_MAX, "parse_ifchanged stuck");

            if self.done() {
                self.warn(
                    format!(
                        "Unclosed ifchanged tag (opened at token {})",
                        start,
                    ),
                    start,
                );
                break;
            }

            let mark = self.position;
            let token = self.current().clone();

            if let Token::BlockEnd { tag, .. } = token {
                if tag == "ifchanged" {
                    self.advance();
                    break;
                }
            }

            if let Token::BlockStart { tag, .. } = token {
                if tag == "else" && !in_else {
                    self.advance();
                    else_branch = Some(Vec::new());
                    in_else = true;
                    continue;
                }
            }

            match self.parse_node()? {
                Some(node) => {
                    if in_else {
                        if let Some(ref mut branch) = else_branch {
                            branch.push(node);
                        }
                    } else {
                        true_branch.push(node);
                    }
                }
                None => {
                    if self.position == mark {
                        self.advance();
                    }
                }
            }

            if self.position == mark {
                self.advance();
            }
        }

        self.leave();

        Ok(Some(AstNode::Ifchanged(Box::new(IfchangedNode {
            raw: raw.to_string(),
            condition: if condition.is_empty() {
                None
            } else {
                Some(condition.to_string())
            },
            true_branch,
            else_branch,
        }))))
    }

    fn parse_filter(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let filter = content.trim_start_matches("filter").trim();
        let body = self.parse_until("filter")?;

        Ok(Some(AstNode::FilterBlock(Box::new(FilterBlockNode {
            raw: raw.to_string(),
            filter: filter.to_string(),
            body,
        }))))
    }

    fn parse_now(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let format = content
            .trim_start_matches("now")
            .trim()
            .trim_matches('\'')
            .trim_matches('"');

        Ok(Some(AstNode::Now(NowNode {
            raw: raw.to_string(),
            format: format.to_string(),
        })))
    }

    fn parse_cycle(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let parts: Vec<&str> = content
            .trim_start_matches("cycle")
            .split_whitespace()
            .collect();

        let mut values = Vec::new();
        let mut as_variable: Option<String> = None;
        let mut output = true;

        let mut index: usize = 0;

        while index < parts.len() {
            if parts[index] == "as" && index + 1 < parts.len() {
                as_variable = Some(parts[index + 1].to_string());
                output = false;
                index += 2;
            } else {
                values.push(
                    parts[index].trim_matches('\'').trim_matches('"').to_string(),
                );

                index += 1;
            }
        }

        Ok(Some(AstNode::Cycle(Box::new(CycleNode {
            raw: raw.to_string(),
            values,
            as_variable,
            output,
        }))))
    }

    fn parse_firstof(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let variables: Vec<String> = content
            .trim_start_matches("firstof")
            .split_whitespace()
            .map(|variable| variable.trim_matches('\'').trim_matches('"').to_string())
            .collect();

        Ok(Some(AstNode::Firstof(FirstofNode {
            raw: raw.to_string(),
            variables,
        })))
    }

    fn parse_widthratio(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let parts: Vec<&str> = content
            .trim_start_matches("widthratio")
            .split_whitespace()
            .collect();

        let value = parts.first().unwrap_or(&"0").to_string();
        let maximum = parts.get(1).unwrap_or(&"0").to_string();
        let divisor = parts.get(2).unwrap_or(&"100").to_string();

        Ok(Some(AstNode::Widthratio(Box::new(WidthratioNode {
            raw: raw.to_string(),
            value,
            maximum,
            divisor,
        }))))
    }

    fn parse_url(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let parts: Vec<&str> = content.split(" as ").collect();

        if parts.len() == 2 {
            let content = parts[0].trim();
            let variable_name = parts[1].trim();

            Ok(Some(AstNode::CaptureAs(Box::new(CaptureAsNode {
                raw: raw.to_string(),
                tag: "url".to_string(),
                content: content.to_string(),
                variable_name: variable_name.to_string(),
            }))))
        } else {
            Ok(Some(AstNode::Text(TextNode {
                content: raw.to_string(),
            })))
        }
    }

    fn parse_static_capture(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let parts: Vec<&str> = content.split(" as ").collect();

        if parts.len() == 2 {
            let content = parts[0].trim();
            let variable_name = parts[1].trim();

            Ok(Some(AstNode::CaptureAs(Box::new(CaptureAsNode {
                raw: raw.to_string(),
                tag: "static".to_string(),
                content: content.to_string(),
                variable_name: variable_name.to_string(),
            }))))
        } else {
            self.parse_static(content, raw)
        }
    }

    fn parse_cache(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let inner = content.trim_start_matches("cache").trim();
        let parts: Vec<&str> = inner.split_whitespace().collect();
        let timeout = parts.first().unwrap_or(&"").to_string();
        let name = parts.get(1).unwrap_or(&"").to_string();
        let body = self.parse_until("cache")?;

        Ok(Some(AstNode::Cache(Box::new(CacheNode {
            raw: raw.to_string(),
            timeout,
            name,
            body,
        }))))
    }

    fn parse_localize(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let value = content.trim_start_matches("localize").trim();
        let body = self.parse_until("localize")?;

        Ok(Some(AstNode::Localize(Box::new(LocalizeNode {
            raw: raw.to_string(),
            value: value.to_string(),
            body,
        }))))
    }

    fn parse_localtime(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let value = content.trim_start_matches("localtime").trim();
        let body = self.parse_until("localtime")?;

        Ok(Some(AstNode::Localtime(Box::new(LocaltimeNode {
            raw: raw.to_string(),
            value: value.to_string(),
            body,
        }))))
    }

    fn parse_spaceless(
        &mut self,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let body = self.parse_until("spaceless")?;

        Ok(Some(AstNode::Spaceless(SpacelessNode {
            raw: raw.to_string(),
            body,
        })))
    }

    fn parse_timezone(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let timezone = content
            .trim_start_matches("timezone")
            .trim()
            .trim_matches('\'')
            .trim_matches('"');

        let body = self.parse_until("timezone")?;

        Ok(Some(AstNode::Timezone(Box::new(TimezoneNode {
            raw: raw.to_string(),
            timezone: timezone.to_string(),
            body,
        }))))
    }

    fn parse_utc(
        &mut self,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let body = self.parse_until("utc")?;

        Ok(Some(AstNode::Utc(UtcNode {
            raw: raw.to_string(),
            body,
        })))
    }

    fn parse_get_static_prefix(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let variable_name = if content.contains(" as ") {
            let parts: Vec<&str> = content.split(" as ").collect();
            Some(parts.get(1).unwrap_or(&"").trim().to_string())
        } else {
            None
        };

        Ok(Some(AstNode::GetStaticPrefix(GetStaticPrefixNode {
            raw: raw.to_string(),
            variable_name,
        })))
    }

    fn parse_get_media_prefix(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let variable_name = if content.contains(" as ") {
            let parts: Vec<&str> = content.split(" as ").collect();
            Some(parts.get(1).unwrap_or(&"").trim().to_string())
        } else {
            None
        };

        Ok(Some(AstNode::GetMediaPrefix(GetMediaPrefixNode {
            raw: raw.to_string(),
            variable_name,
        })))
    }

    fn parse_resetcycle(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let name = content.trim_start_matches("resetcycle").trim();

        Ok(Some(AstNode::Resetcycle(ResetcycleNode {
            raw: raw.to_string(),
            name: name.to_string(),
        })))
    }

    fn parse_lorem(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let parts: Vec<&str> = content
            .trim_start_matches("lorem")
            .split_whitespace()
            .collect();

        let count = parts.first().unwrap_or(&"1").to_string();
        let method = parts.get(1).unwrap_or(&"i").to_string();
        let random = parts.get(2).is_some_and(|&string| string == "r");

        Ok(Some(AstNode::Lorem(Box::new(LoremNode {
            raw: raw.to_string(),
            count,
            method,
            random,
        }))))
    }

    fn parse_querystring(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let parts: Vec<&str> = content.split(" as ").collect();

        let body = parts
            .first()
            .unwrap_or(&"")
            .trim_start_matches("querystring")
            .trim();

        let variable_name = if parts.len() == 2 {
            Some(parts.get(1).unwrap_or(&"").trim().to_string())
        } else {
            None
        };

        let mut parameters = Vec::new();

        for pair in body.split_whitespace() {
            if let Some((key, value)) = pair.split_once('=') {
                parameters.push(Binding {
                    name: key.to_string(),
                    value: value.to_string(),
                });
            }
        }

        Ok(Some(AstNode::Querystring(Box::new(QuerystringNode {
            raw: raw.to_string(),
            parameters,
            variable_name,
        }))))
    }

    fn parse_translate(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let parts: Vec<&str> = content.split(" as ").collect();

        let body = parts
            .first()
            .unwrap_or(&"")
            .trim_start_matches("translate")
            .trim();

        let variable_name = if parts.len() == 2 {
            Some(parts.get(1).unwrap_or(&"").trim().to_string())
        } else {
            None
        };

        let message = body.trim_matches('\'').trim_matches('"');
        let noop = body.contains("noop");

        Ok(Some(AstNode::Translate(Box::new(TranslateNode {
            raw: raw.to_string(),
            message: message.to_string(),
            variable_name,
            noop,
        }))))
    }

    fn parse_plural(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let content = content.trim_start_matches("plural").trim();

        Ok(Some(AstNode::Plural(PluralNode {
            raw: raw.to_string(),
            content: content.to_string(),
        })))
    }

    fn parse_regroup(
        &mut self,
        content: &str,
        raw: &str,
    ) -> Result<Option<AstNode>, ParseError> {
        let inner = content.trim_start_matches("regroup").trim();
        let parts: Vec<&str> = inner.split_whitespace().collect();
        let list = parts.first().unwrap_or(&"").to_string();
        let field = parts.get(2).unwrap_or(&"").to_string();
        let as_variable = parts.get(4).unwrap_or(&"").to_string();

        Ok(Some(AstNode::Regroup(Box::new(RegroupNode {
            raw: raw.to_string(),
            list,
            field,
            as_variable,
        }))))
    }

    fn try_json_script(
        &self,
        expression: &str,
        filters: &[Filter],
        raw: &str,
    ) -> Option<AstNode> {
        let json_script_filter =
            filters.iter().find(|filter| filter.name == "json_script")?;

        let id = json_script_filter
            .arguments
            .first()?
            .trim_matches('\'')
            .trim_matches('"');

        let variable = expression.split('|').next()?.trim().to_string();

        Some(AstNode::JsonScript(Box::new(JsonScriptNode {
            raw: raw.to_string(),
            variable,
            id: id.to_string(),
        })))
    }
}

pub fn parse(content: &str) -> Result<ParseOutput, ParseError> {
    let mut lexer = Lexer::new(content);
    let tokens = lexer
        .tokenize()
        .map_err(|error| ParseError::Generic(error.to_string()))?;

    debug_assert!(
        !tokens.is_empty(),
        "lexer must produce at least one token (Eof)",
    );

    let mut parser = Parser::new(tokens);
    let nodes = parser.parse()?;

    Ok(ParseOutput {
        nodes,
        diagnostics: parser.diagnostics,
    })
}
