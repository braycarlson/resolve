use crate::ast::*;


const STACK_CAPACITY_INITIAL: u32 = 64;
const OUTPUT_CAPACITY_PER_NODE: u32 = 64;
const WORK_ITERATIONS_MAX: u32 = 10_000_000;
const COLLAPSE_LENGTH_MAX: u32 = 50_000_000;

enum Work<'a> {
    Node(&'a AstNode),
    Literal(&'a str),
}

pub fn generate(nodes: &[AstNode]) -> String {
    assert!(
        (nodes.len() as u32) <= WORK_ITERATIONS_MAX,
        "codegen input node count {} exceeds maximum {}",
        nodes.len(),
        WORK_ITERATIONS_MAX,
    );

    let count = nodes.len();

    let mut output = String::with_capacity(
        count.saturating_mul(OUTPUT_CAPACITY_PER_NODE as usize),
    );

    let mut stack: Vec<Work> = Vec::with_capacity(STACK_CAPACITY_INITIAL as usize);

    spread(nodes, &mut stack);

    let mut iterations: u32 = 0;

    while let Some(work) = stack.pop() {
        iterations += 1;

        assert!(
            iterations <= WORK_ITERATIONS_MAX,
            "codegen exceeded {WORK_ITERATIONS_MAX} iterations",
        );

        match work {
            Work::Literal(string) => output.push_str(string),
            Work::Node(node) => enqueue(node, &mut stack),
        }
    }

    collapse(&output)
}

fn collapse(input: &str) -> String {
    assert!(
        (input.len() as u32) <= COLLAPSE_LENGTH_MAX,
        "collapse input length {} exceeds maximum {}",
        input.len(),
        COLLAPSE_LENGTH_MAX,
    );

    let bytes = input.as_bytes();
    let length = bytes.len();

    let mut result = String::with_capacity(length);
    let mut was_blank = false;
    let mut first = true;
    let mut position: usize = 0;

    while position <= length {
        let start = position;

        while position < length && bytes[position] != b'\n' {
            position += 1;
        }

        let line = &input[start..position];

        if blank(&bytes[start..position]) {
            if !was_blank && !first {
                result.push('\n');
            }

            was_blank = true;
        } else {
            was_blank = false;

            if !first {
                result.push('\n');
            }

            first = false;
            result.push_str(line);
        }

        position += 1;
    }

    if result.ends_with('\n') {
        result.truncate(result.len() - 1);
    }

    if !input.is_empty() && input.ends_with('\n') {
        result.push('\n');
    }

    debug_assert!(
        result.len() <= input.len(),
        "collapse must not produce output longer than input",
    );

    result
}

#[inline]
fn blank(line: &[u8]) -> bool {
    line.iter().all(|&byte| byte == b' ' || byte == b'\t' || byte == b'\r')
}

#[inline(always)]
fn spread<'a>(nodes: &'a [AstNode], stack: &mut Vec<Work<'a>>) {
    for node in nodes.iter().rev() {
        stack.push(Work::Node(node));
    }
}

fn enqueue<'a>(node: &'a AstNode, stack: &mut Vec<Work<'a>>) {
    match node {
        AstNode::Text(text) => stack.push(Work::Literal(&text.content)),
        AstNode::Variable(variable) => stack.push(Work::Literal(&variable.raw)),
        AstNode::Block(block) => spread(&block.content, stack),

        AstNode::If(if_node) => enqueue_if(if_node, stack),
        AstNode::For(for_node) => enqueue_for(for_node, stack),
        AstNode::Ifchanged(ifchanged) => enqueue_ifchanged(ifchanged, stack),

        AstNode::CommentBlock(comment_block) => {
            enqueue_bookend(
                &comment_block.raw,
                &comment_block.content,
                "{% endcomment %}",
                stack,
            );
        }

        AstNode::Blocktranslate(blocktranslate) => {
            enqueue_body(
                &blocktranslate.raw,
                &blocktranslate.body,
                "{% endblocktranslate %}",
                stack,
            );
        }

        AstNode::Verbatim(verbatim) => {
            enqueue_bookend(
                &verbatim.raw,
                &verbatim.content,
                "{% endverbatim %}",
                stack,
            );
        }

        AstNode::With(with) => {
            enqueue_body(&with.raw, &with.body, "{% endwith %}", stack);
        }

        AstNode::Autoescape(autoescape) => {
            enqueue_body(
                &autoescape.raw,
                &autoescape.body,
                "{% endautoescape %}",
                stack,
            );
        }

        AstNode::Language(language) => {
            enqueue_body(
                &language.raw,
                &language.body,
                "{% endlanguage %}",
                stack,
            );
        }

        AstNode::FilterBlock(filter_block) => {
            enqueue_body(
                &filter_block.raw,
                &filter_block.body,
                "{% endfilter %}",
                stack,
            );
        }

        AstNode::Cache(cache) => {
            enqueue_body(&cache.raw, &cache.body, "{% endcache %}", stack);
        }

        AstNode::Localize(localize) => {
            enqueue_body(
                &localize.raw,
                &localize.body,
                "{% endlocalize %}",
                stack,
            );
        }

        AstNode::Localtime(localtime) => {
            enqueue_body(
                &localtime.raw,
                &localtime.body,
                "{% endlocaltime %}",
                stack,
            );
        }

        AstNode::Spaceless(spaceless) => {
            enqueue_body(
                &spaceless.raw,
                &spaceless.body,
                "{% endspaceless %}",
                stack,
            );
        }

        AstNode::Timezone(timezone) => {
            enqueue_body(
                &timezone.raw,
                &timezone.body,
                "{% endtimezone %}",
                stack,
            );
        }

        AstNode::Utc(utc) => {
            enqueue_body(&utc.raw, &utc.body, "{% endutc %}", stack);
        }

        AstNode::Extends(_)
        | AstNode::Include(_)
        | AstNode::Load(_)
        | AstNode::Static(_)
        | AstNode::Csrftoken(_)
        | AstNode::Trans(_)
        | AstNode::TemplateTag(_)
        | AstNode::Now(_)
        | AstNode::Cycle(_)
        | AstNode::Firstof(_)
        | AstNode::Widthratio(_)
        | AstNode::JsonScript(_)
        | AstNode::CaptureAs(_)
        | AstNode::Debug(_)
        | AstNode::GetStaticPrefix(_)
        | AstNode::GetMediaPrefix(_)
        | AstNode::Resetcycle(_)
        | AstNode::Lorem(_)
        | AstNode::Querystring(_)
        | AstNode::Translate(_)
        | AstNode::Plural(_)
        | AstNode::Regroup(_) => enqueue_raw(node, stack),
    }
}

#[inline]
fn enqueue_raw<'a>(node: &'a AstNode, stack: &mut Vec<Work<'a>>) {
    let raw: &str = match node {
        AstNode::Extends(extends) => &extends.raw,
        AstNode::Include(include) => &include.raw,
        AstNode::Load(load) => &load.raw,
        AstNode::Static(static_node) => &static_node.raw,
        AstNode::Csrftoken(csrftoken) => &csrftoken.raw,
        AstNode::Trans(trans) => &trans.raw,
        AstNode::TemplateTag(templatetag) => &templatetag.raw,
        AstNode::Now(now) => &now.raw,
        AstNode::Cycle(cycle) => &cycle.raw,
        AstNode::Firstof(firstof) => &firstof.raw,
        AstNode::Widthratio(widthratio) => &widthratio.raw,
        AstNode::JsonScript(json_script) => &json_script.raw,
        AstNode::CaptureAs(capture_as) => &capture_as.raw,
        AstNode::Debug(debug) => &debug.raw,
        AstNode::GetStaticPrefix(get_static_prefix) => &get_static_prefix.raw,
        AstNode::GetMediaPrefix(get_media_prefix) => &get_media_prefix.raw,
        AstNode::Resetcycle(resetcycle) => &resetcycle.raw,
        AstNode::Lorem(lorem) => &lorem.raw,
        AstNode::Querystring(querystring) => &querystring.raw,
        AstNode::Translate(translate) => &translate.raw,
        AstNode::Plural(plural) => &plural.raw,
        AstNode::Regroup(regroup) => &regroup.raw,
        _ => unreachable!("variant handled by enqueue directly")
    };

    stack.push(Work::Literal(raw));
}

#[inline]
fn enqueue_if<'a>(node: &'a IfNode, stack: &mut Vec<Work<'a>>) {
    stack.push(Work::Literal("{% endif %}"));

    if let Some(else_branch) = &node.else_branch {
        spread(else_branch, stack);
        stack.push(Work::Literal("{% else %}"));
    }

    for elif in node.elif_branches.iter().rev() {
        spread(&elif.body, stack);
        stack.push(Work::Literal(" %}"));
        stack.push(Work::Literal(elif.condition.as_str()));
        stack.push(Work::Literal("{% elif "));
    }

    spread(&node.true_branch, stack);
    stack.push(Work::Literal(&node.raw));
}

#[inline]
fn enqueue_for<'a>(node: &'a ForNode, stack: &mut Vec<Work<'a>>) {
    stack.push(Work::Literal("{% endfor %}"));

    if let Some(empty_branch) = &node.empty_branch {
        spread(empty_branch, stack);
        stack.push(Work::Literal("{% empty %}"));
    }

    spread(&node.body, stack);
    stack.push(Work::Literal(&node.raw));
}

#[inline]
fn enqueue_ifchanged<'a>(node: &'a IfchangedNode, stack: &mut Vec<Work<'a>>) {
    stack.push(Work::Literal("{% endifchanged %}"));

    if let Some(else_branch) = &node.else_branch {
        spread(else_branch, stack);
        stack.push(Work::Literal("{% else %}"));
    }

    spread(&node.true_branch, stack);
    stack.push(Work::Literal(&node.raw));
}

#[inline]
fn enqueue_body<'a>(
    raw: &'a str,
    body: &'a [AstNode],
    end_tag: &'static str,
    stack: &mut Vec<Work<'a>>,
) {
    stack.push(Work::Literal(end_tag));
    spread(body, stack);
    stack.push(Work::Literal(raw));
}

#[inline]
fn enqueue_bookend<'a>(
    raw: &'a str,
    content: &'a str,
    end_tag: &'static str,
    stack: &mut Vec<Work<'a>>,
) {
    stack.push(Work::Literal(end_tag));
    stack.push(Work::Literal(content));
    stack.push(Work::Literal(raw));
}
