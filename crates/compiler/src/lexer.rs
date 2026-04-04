use crate::error::LexError;

const TOKEN_COUNT_MAX: u32 = 500_000;
const FILTER_COUNT_MAX: u32 = 100;
const FILTER_ARGUMENTS_ITERATIONS_MAX: u32 = 10_000;
const TOKEN_CAPACITY_DIVISOR: u32 = 8;
const ENDVERBATIM_TAG: &str = "endverbatim";

#[derive(Debug, Clone, PartialEq)]
pub enum Token<'a> {
    Text(&'a str),

    Variable {
        raw: &'a str,
        expression: &'a str,
        filters: Vec<Filter<'a>>,
    },

    BlockStart {
        raw: &'a str,
        tag: &'a str,
        content: &'a str,
    },

    BlockEnd {
        tag: &'a str,
        raw: &'a str,
    },

    Comment(&'a str),

    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Filter<'a> {
    pub name: &'a str,
    pub arguments: Vec<&'a str>,
}

pub struct Lexer<'a> {
    input: &'a str,
    bytes: &'a [u8],
    position: u32,
    length: u32,
    buffer: Option<&'a str>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        assert!(
            input.len() <= u32::MAX as usize,
            "input length exceeds u32 maximum",
        );

        let length = input.len() as u32;

        Self {
            input,
            bytes: input.as_bytes(),
            position: 0,
            length,
            buffer: None,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token<'a>>, LexError> {
        let capacity = (self.length / TOKEN_CAPACITY_DIVISOR).max(16) as usize;
        let mut tokens = Vec::with_capacity(capacity);

        while self.position < self.length || self.buffer.is_some() {
            assert!(
                (tokens.len() as u32) < TOKEN_COUNT_MAX,
                "token count exceeds maximum of {TOKEN_COUNT_MAX}",
            );

            if let Some(token) = self.next_token()? {
                tokens.push(token);
            }
        }

        tokens.push(Token::Eof);

        assert!(
            tokens.last() == Some(&Token::Eof),
            "token stream must end with Eof",
        );

        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Option<Token<'a>>, LexError> {
        if let Some(content) = self.buffer.take() {
            return Ok(Some(Token::Text(content)));
        }

        if self.position >= self.length {
            return Ok(None);
        }

        let byte = self.bytes[self.position as usize];

        if byte == b'{' && self.position + 1 < self.length {
            let next = self.bytes[(self.position + 1) as usize];

            if next == b'{' {
                return self.consume_variable();
            }

            if next == b'%' {
                return self.consume_block();
            }

            if next == b'#' {
                return self.consume_comment();
            }
        }

        self.consume_text()
    }

    fn peek(&self, offset: u32) -> Option<u8> {
        let index = self.position.checked_add(offset)?;

        if index < self.length {
            Some(self.bytes[index as usize])
        } else {
            None
        }
    }

    fn consume_variable(&mut self) -> Result<Option<Token<'a>>, LexError> {
        let start = self.position;
        self.position += 2;

        let mut quoted = false;
        let mut quote: u8 = 0;

        while self.position < self.length {
            let byte = self.bytes[self.position as usize];

            if quoted {
                if byte == b'\\' {
                    self.position += 1;

                    if self.position < self.length {
                        self.position += 1;
                    }

                    continue;
                }

                if byte == quote {
                    quoted = false;
                }

                self.position += 1;
            } else if byte == b'"' || byte == b'\'' {
                quoted = true;
                quote = byte;
                self.position += 1;
            } else if byte == b'}' && self.peek(1) == Some(b'}') {
                let inner = &self.input[(start + 2) as usize..self.position as usize];
                self.position += 2;

                let raw = &self.input[start as usize..self.position as usize];
                let expression = inner.trim();
                let filters = parse_filters(expression);

                assert!(raw.starts_with("{{"), "variable raw must start with {{{{");
                assert!(raw.ends_with("}}"), "variable raw must end with }}}}");

                return Ok(Some(Token::Variable {
                    raw,
                    expression,
                    filters,
                }));
            } else {
                self.position += 1;
            }
        }

        Err(LexError::UnterminatedVariable(start))
    }

    fn consume_block(&mut self) -> Result<Option<Token<'a>>, LexError> {
        let start = self.position;
        self.position += 2;

        let inner = self.position;

        while self.position < self.length {
            if self.bytes[self.position as usize] == b'%' && self.peek(1) == Some(b'}') {
                let end = self.position;
                self.position += 2;

                let raw = &self.input[start as usize..self.position as usize];
                let content = self.input[inner as usize..end as usize].trim();

                assert!(raw.starts_with("{%"), "block raw must start with {{%}}");
                assert!(raw.ends_with("%}"), "block raw must end with %}}");

                if content == "verbatim" || content.starts_with("verbatim ") {
                    return self.handle_verbatim(raw, content);
                }

                if content.starts_with("end") {
                    let tag = content
                        .trim_start_matches("end")
                        .split_whitespace()
                        .next()
                        .unwrap_or("");

                    return Ok(Some(Token::BlockEnd { tag, raw }));
                }

                let tag = content.split_whitespace().next().unwrap_or("");

                return Ok(Some(Token::BlockStart { raw, tag, content }));
            }

            self.position += 1;
        }

        Err(LexError::UnterminatedBlock(start))
    }

    fn handle_verbatim(
        &mut self,
        raw: &'a str,
        content: &'a str,
    ) -> Result<Option<Token<'a>>, LexError> {
        let body = self.consume_verbatim();
        self.buffer = Some(body);

        Ok(Some(Token::BlockStart {
            raw,
            tag: "verbatim",
            content,
        }))
    }

    fn consume_verbatim(&mut self) -> &'a str {
        let start = self.position;

        while self.position + 1 < self.length {
            if self.bytes[self.position as usize] == b'{'
                && self.bytes[(self.position + 1) as usize] == b'%'
            {
                let offset = (self.position + 2) as usize;

                if let Some(end) = self.input[offset..].find("%}") {
                    let inner = self.input[offset..offset + end].trim();

                    if inner == ENDVERBATIM_TAG {
                        let content = &self.input[start as usize..self.position as usize];
                        self.position = (offset + end + 2) as u32;

                        assert!(
                            self.position <= self.length,
                            "verbatim scan must not exceed input length",
                        );

                        return content;
                    }
                }
            }

            self.position += 1;
        }

        let content = &self.input[start as usize..self.length as usize];
        self.position = self.length;
        content
    }

    fn consume_comment(&mut self) -> Result<Option<Token<'a>>, LexError> {
        let start = self.position;
        self.position += 2;

        let inner = self.position;

        while self.position < self.length {
            if self.bytes[self.position as usize] == b'#' && self.peek(1) == Some(b'}') {
                let content = &self.input[inner as usize..self.position as usize];
                self.position += 2;

                assert!(
                    self.position > start,
                    "consume_comment must advance position",
                );

                return Ok(Some(Token::Comment(content)));
            }

            self.position += 1;
        }

        Err(LexError::UnterminatedComment(start))
    }

    fn consume_text(&mut self) -> Result<Option<Token<'a>>, LexError> {
        let start = self.position;

        while self.position < self.length {
            let byte = self.bytes[self.position as usize];

            if byte == b'{' && self.position + 1 < self.length {
                let next = self.bytes[(self.position + 1) as usize];

                if next == b'{' || next == b'%' || next == b'#' {
                    break;
                }
            }

            self.position += 1;
        }

        if self.position == start {
            return Ok(None);
        }

        let text = &self.input[start as usize..self.position as usize];

        assert!(!text.is_empty(), "consume_text must produce non-empty text",);

        Ok(Some(Token::Text(text)))
    }
}

fn parse_filters<'a>(expression: &'a str) -> Vec<Filter<'a>> {
    let bytes = expression.as_bytes();
    let length = bytes.len();

    let mut filters = Vec::new();
    let mut quoted = false;
    let mut quote: u8 = 0;
    let mut piped = false;
    let mut start: usize = 0;
    let mut position: usize = 0;

    while position < length {
        assert!(
            (filters.len() as u32) <= FILTER_COUNT_MAX,
            "filter count exceeds maximum of {FILTER_COUNT_MAX}",
        );

        let byte = bytes[position];

        if quoted {
            if byte == quote {
                quoted = false;
            }

            position += 1;
        } else if byte == b'"' || byte == b'\'' {
            quoted = true;
            quote = byte;
            position += 1;
        } else if byte == b'|' {
            if !piped {
                piped = true;
                start = position + 1;
            } else {
                let segment = expression[start..position].trim();

                if let Some(filter) = parse_filter(segment) {
                    filters.push(filter);
                }

                start = position + 1;
            }

            position += 1;
        } else {
            position += 1;
        }
    }

    if piped {
        let segment = expression[start..].trim();

        if let Some(filter) = parse_filter(segment) {
            filters.push(filter);
        }
    }

    filters
}

fn parse_filter<'a>(input: &'a str) -> Option<Filter<'a>> {
    if input.is_empty() {
        return None;
    }

    let (name, arguments) = match input.find(':') {
        Some(colon) => {
            let name = input[..colon].trim();
            let remainder = &input[colon + 1..];

            let arguments = split_arguments(remainder);
            (name, arguments)
        }

        None => (input.trim(), Vec::new()),
    };

    if name.is_empty() {
        return None;
    }

    Some(Filter { name, arguments })
}

fn split_arguments(input: &str) -> Vec<&str> {
    let bytes = input.as_bytes();
    let length = bytes.len();

    if length == 0 {
        return Vec::new();
    }

    let mut arguments = Vec::new();
    let mut quoted = false;
    let mut quote: u8 = 0;
    let mut start: usize = 0;
    let mut position: usize = 0;
    let mut iterations: u32 = 0;

    while position < length {
        iterations += 1;

        assert!(
            iterations <= FILTER_ARGUMENTS_ITERATIONS_MAX,
            "split_arguments exceeded maximum iterations",
        );

        let byte = bytes[position];

        if quoted {
            if byte == quote {
                quoted = false;
            }
        } else if byte == b'"' || byte == b'\'' {
            quoted = true;
            quote = byte;
        } else if byte == b',' {
            let segment = input[start..position].trim();

            if !segment.is_empty() {
                arguments.push(segment);
            }

            start = position + 1;
        }

        position += 1;
    }

    let trailing = input[start..].trim();

    if !trailing.is_empty() {
        arguments.push(trailing);
    }

    arguments
}
