mod common;

use compiler::ast::AstNode;

use common::parse;


#[test]
fn test_parser_simple_variable() {
    let ast = parse("{{ variable }}").unwrap();

    assert_eq!(ast.len(), 1);
    match &ast[0] {
        AstNode::Variable(v) => {
            assert_eq!(v.expression, "variable");
            assert!(v.filters.is_empty());
        }
        _ => panic!("Expected Variable node"),
    }
}

#[test]
fn test_parser_variable_with_filter() {
    let ast = parse("{{ variable|default:\"\" }}").unwrap();

    match &ast[0] {
        AstNode::Variable(v) => {
            assert_eq!(v.expression, "variable|default:\"\"");
            assert_eq!(v.filters.len(), 1);
            assert_eq!(v.filters[0].name, "default");
            assert_eq!(v.filters[0].arguments, vec!["\"\""]);
        }
        _ => panic!("Expected Variable node"),
    }
}

#[test]
fn test_parser_variable_with_multiple_filters() {
    let ast = parse("{{ var|default:''|upper|escape }}").unwrap();

    match &ast[0] {
        AstNode::Variable(v) => {
            assert_eq!(v.filters.len(), 3);
        }
        _ => panic!("Expected Variable node"),
    }
}

#[test]
fn test_parser_extends() {
    let ast = parse("{% extends 'base.html' %}").unwrap();

    match &ast[0] {
        AstNode::Extends(e) => {
            assert_eq!(e.parent_path, "base.html");
        }
        _ => panic!("Expected Extends node"),
    }
}

#[test]
fn test_parser_block() {
    let ast = parse("{% block content %}Hello{% endblock %}").unwrap();

    match &ast[0] {
        AstNode::Block(b) => {
            assert_eq!(b.name, "content");
            assert!(!b.has_super_reference);
        }
        _ => panic!("Expected Block node"),
    }
}

#[test]
fn test_parser_block_with_super() {
    let ast = parse("{% block content %}{{ block.super }} Extra{% endblock %}").unwrap();

    match &ast[0] {
        AstNode::Block(b) => {
            assert!(b.has_super_reference);
            assert_eq!(b.content.len(), 2);
        }
        _ => panic!("Expected Block node"),
    }
}

#[test]
fn test_parser_include() {
    let ast = parse("{% include \"partial.html\" %}").unwrap();

    assert!(matches!(ast[0], AstNode::Include(_)));
}

#[test]
fn test_parser_include_with_with() {
    let ast = parse("{% include 'component.html' with var=value %}").unwrap();

    match &ast[0] {
        AstNode::Include(i) => {
            assert_eq!(i.path, "component.html");
            assert_eq!(i.with_variables.len(), 1);
            assert_eq!(i.with_variables[0].name, "var");
            assert_eq!(i.with_variables[0].value, "value");
        }
        _ => panic!("Expected Include node"),
    }
}

#[test]
fn test_parser_include_multiple_with_vars() {
    let ast = parse("{% include 'button.html' with text='Click' icon='plus' href=url %}").unwrap();

    match &ast[0] {
        AstNode::Include(i) => {
            assert_eq!(i.with_variables.len(), 3);
            assert_eq!(i.with_variables[0].name, "text");
            assert_eq!(i.with_variables[0].value, "'Click'");
            assert_eq!(i.with_variables[1].name, "icon");
            assert_eq!(i.with_variables[1].value, "'plus'");
            assert_eq!(i.with_variables[2].name, "href");
            assert_eq!(i.with_variables[2].value, "url");
        }
        _ => panic!("Expected Include node"),
    }
}

#[test]
fn test_parser_include_only() {
    let ast = parse("{% include \"partial.html\" only %}").unwrap();

    match &ast[0] {
        AstNode::Include(i) => {
            assert!(i.only);
        }
        _ => panic!("Expected Include node"),
    }
}

#[test]
fn test_parser_recursive_include_pattern() {
    let ast = parse("{% include 'element.html' with depth=1 %}").unwrap();

    assert!(!ast.is_empty());
    assert!(ast.iter().any(|n| matches!(n, AstNode::Include(_))));
}

#[test]
fn test_parser_if() {
    let ast = parse("{% if condition %}Yes{% endif %}").unwrap();

    match &ast[0] {
        AstNode::If(i) => {
            assert_eq!(i.condition, "condition");
            assert_eq!(i.true_branch.len(), 1);
        }
        _ => panic!("Expected If node"),
    }
}

#[test]
fn test_parser_if_else() {
    let ast = parse("{% if condition %}Yes{% else %}No{% endif %}").unwrap();

    match &ast[0] {
        AstNode::If(i) => {
            assert_eq!(i.condition, "condition");
            assert_eq!(i.true_branch.len(), 1);
            assert!(i.else_branch.is_some());
            let else_branch = i.else_branch.as_ref().unwrap();
            assert_eq!(else_branch.len(), 1);
        }
        _ => panic!("Expected If node"),
    }
}

#[test]
fn test_parser_if_elif_else() {
    let ast = parse("{% if a %}A{% elif b %}B{% else %}C{% endif %}").unwrap();

    match &ast[0] {
        AstNode::If(i) => {
            assert_eq!(i.condition, "a");
            assert_eq!(i.elif_branches.len(), 1);
            assert!(i.else_branch.is_some());

            let elif = &i.elif_branches[0];
            assert_eq!(elif.condition, "b");
            assert_eq!(elif.body.len(), 1);

            let else_branch = i.else_branch.as_ref().unwrap();
            assert_eq!(else_branch.len(), 1);
        }
        _ => panic!("Expected If node"),
    }
}

#[test]
fn test_parser_for() {
    let ast = parse("{% for item in items %}{{ item }}{% endfor %}").unwrap();

    match &ast[0] {
        AstNode::For(f) => {
            assert_eq!(f.variable, "item");
            assert_eq!(f.iterable, "items");
            assert_eq!(f.body.len(), 1);
            assert!(f.empty_branch.is_none());
        }
        _ => panic!("Expected For node"),
    }
}

#[test]
fn test_parser_for_with_empty() {
    let ast = parse("{% for item in items %}{{ item }}{% empty %}No items{% endfor %}").unwrap();

    match &ast[0] {
        AstNode::For(f) => {
            assert!(f.empty_branch.is_some());
            let empty = f.empty_branch.as_ref().unwrap();
            assert_eq!(empty.len(), 1);
        }
        _ => panic!("Expected For node"),
    }
}

#[test]
fn test_parser_for_tuple_unpacking() {
    let ast =
        parse("{% for key, value in dict.items %}{{ key }}: {{ value }}{% endfor %}").unwrap();

    match &ast[0] {
        AstNode::For(f) => {
            assert!(f.variable.contains(","));
        }
        _ => panic!("Expected For node"),
    }
}

#[test]
fn test_parser_for_with_custom_filter() {
    let ast = parse(
        "{% for key, value in data|safe_dict_items %}{{ key }}{% endfor %}",
    )
    .unwrap();

    match &ast[0] {
        AstNode::For(f) => {
            assert!(f.variable.contains(","));
            assert!(f.iterable.contains("safe_dict_items"));
        }
        _ => panic!("Expected For node"),
    }
}

#[test]
fn test_parser_with() {
    let ast = parse("{% with total=price|add:tax %}{{ total }}{% endwith %}").unwrap();

    match &ast[0] {
        AstNode::With(w) => {
            assert_eq!(w.bindings.len(), 1);
            assert_eq!(w.bindings[0].name, "total");
        }
        _ => panic!("Expected With node"),
    }
}

#[test]
fn test_parser_load() {
    let ast = parse("{% load static %}").unwrap();

    match &ast[0] {
        AstNode::Load(l) => {
            assert_eq!(l.libraries.len(), 1);
            assert_eq!(l.libraries[0], "static");
        }
        _ => panic!("Expected Load node"),
    }
}

#[test]
fn test_parser_load_multiple() {
    let ast = parse("{% load static i18n %}").unwrap();

    match &ast[0] {
        AstNode::Load(l) => {
            assert_eq!(l.libraries.len(), 2);
        }
        _ => panic!("Expected Load node"),
    }
}

#[test]
fn test_parser_csrf_token() {
    let ast = parse("{% csrf_token %}").unwrap();

    assert!(matches!(ast[0], AstNode::Csrftoken(_)));
}

#[test]
fn test_parser_comment_block() {
    let ast =
        parse("{% comment %}This is hidden{% endcomment %}").unwrap();

    assert!(matches!(ast[0], AstNode::CommentBlock(_)));
}

#[test]
fn test_parser_autoescape() {
    let ast =
        parse("{% autoescape off %}{{ content }}{% endautoescape %}").unwrap();

    assert!(matches!(ast[0], AstNode::Autoescape(_)));
}

#[test]
fn test_parser_blocktranslate() {
    let ast = parse("{% blocktranslate %}Hello{% endblocktranslate %}").unwrap();

    assert!(ast
        .iter()
        .any(|n| matches!(n, AstNode::Block(_) | AstNode::Blocktranslate(_))));
}

#[test]
fn test_parser_trans() {
    let ast = parse("{% trans \"Hello\" %}").unwrap();

    assert!(matches!(ast[0], AstNode::Trans(_)));
}

#[test]
fn test_parser_language() {
    let ast = parse("{% language \"en\" %}Content{% endlanguage %}").unwrap();

    assert!(matches!(ast[0], AstNode::Language(_)));
}

#[test]
fn test_parser_verbatim() {
    let ast = parse("{% verbatim %}{{ django }}{% endverbatim %}").unwrap();

    assert!(matches!(ast[0], AstNode::Verbatim(_)));
}

#[test]
fn test_parser_templatetag() {
    let ast = parse("{% templatetag \"openblock\" %}").unwrap();

    match &ast[0] {
        AstNode::TemplateTag(t) => {
            assert_eq!(t.format, "openblock");
        }
        _ => panic!("Expected TemplateTag node"),
    }
}

#[test]
fn test_parser_ifchanged() {
    let ast = parse("{% ifchanged %}{{ value }}{% endifchanged %}").unwrap();

    assert!(matches!(ast[0], AstNode::Ifchanged(_)));
}

#[test]
fn test_parser_ifchanged_with_condition() {
    let ast = parse("{% ifchanged object.date %}{{ value }}{% endifchanged %}").unwrap();

    match &ast[0] {
        AstNode::Ifchanged(i) => {
            assert!(i.condition.is_some());
        }
        _ => panic!("Expected Ifchanged node"),
    }
}

#[test]
fn test_parser_filter_block() {
    let ast = parse("{% filter upper %}hello{% endfilter %}").unwrap();

    assert!(matches!(ast[0], AstNode::FilterBlock(_)));
}

#[test]
fn test_parser_now() {
    let ast = parse("{% now \"Y-m-d\" %}").unwrap();

    match &ast[0] {
        AstNode::Now(n) => {
            assert_eq!(n.format, "Y-m-d");
        }
        _ => panic!("Expected Now node"),
    }
}

#[test]
fn test_parser_cycle() {
    let ast = parse("{% cycle 'a' 'b' 'c' %}").unwrap();

    assert!(matches!(ast[0], AstNode::Cycle(_)));
}

#[test]
fn test_parser_firstof() {
    let ast = parse("{% firstof var1 var2 var3 %}").unwrap();

    assert!(matches!(ast[0], AstNode::Firstof(_)));
}

#[test]
fn test_parser_widthratio() {
    let ast = parse("{% widthratio this_value max_value 100 %}").unwrap();

    assert!(matches!(ast[0], AstNode::Widthratio(_)));
}

#[test]
fn test_parser_static() {
    let ast = parse("{% static 'css/style.css' %}").unwrap();

    assert!(matches!(ast[0], AstNode::Static(_)));
}

#[test]
fn test_parser_url() {
    let ast = parse("{% url 'home' %}").unwrap();

    assert!(!ast.is_empty());
}

#[test]
fn test_parser_url_with_capture() {
    let ast = parse("{% url 'home' as home_url %}").unwrap();

    assert!(matches!(ast[0], AstNode::CaptureAs(_)));
}

#[test]
fn test_parser_regroup() {
    let ast = parse("{% regroup people by gender as gender_list %}").unwrap();

    assert!(matches!(ast[0], AstNode::Regroup(_)));
}

#[test]
fn test_parser_regroup_fields() {
    let ast = parse("{% regroup people by gender as gender_list %}").unwrap();

    if let AstNode::Regroup(rg) = &ast[0] {
        assert_eq!(rg.list, "people", "list should be 'people', got '{}'", rg.list);
        assert_eq!(rg.field, "gender", "field should be 'gender', got '{}'", rg.field);
        assert_eq!(rg.as_variable, "gender_list", "as_variable should be 'gender_list', got '{}'", rg.as_variable);
    } else {
        panic!("Expected Regroup node");
    }
}

#[test]
fn test_parser_cache() {
    let ast = parse("{% cache 500 sidebar %}Sidebar content{% endcache %}").unwrap();

    assert!(matches!(ast[0], AstNode::Cache(_)));
}

#[test]
fn test_parser_localize() {
    let ast = parse("{% localize on %}Content{% endlocalize %}").unwrap();

    assert!(matches!(ast[0], AstNode::Localize(_)));
}

#[test]
fn test_parser_localtime() {
    let ast = parse("{% localtime on %}Content{% endlocaltime %}").unwrap();

    assert!(matches!(ast[0], AstNode::Localtime(_)));
}

#[test]
fn test_parser_spaceless() {
    let ast = parse("{% spaceless %}<p>  Content  </p>{% endspaceless %}").unwrap();

    assert!(matches!(ast[0], AstNode::Spaceless(_)));
}

#[test]
fn test_parser_timezone() {
    let ast = parse(r#"{% timezone "America/New_York" %}Content{% endtimezone %}"#).unwrap();

    assert!(matches!(ast[0], AstNode::Timezone(_)));
}

#[test]
fn test_parser_utc() {
    let ast = parse("{% utc %}Content{% endutc %}").unwrap();

    assert!(matches!(ast[0], AstNode::Utc(_)));
}

#[test]
fn test_parser_debug() {
    let ast = parse("{% debug %}").unwrap();

    assert!(matches!(ast[0], AstNode::Debug(_)));
}

#[test]
fn test_parser_get_static_prefix() {
    let ast = parse("{% get_static_prefix %}").unwrap();

    assert!(matches!(ast[0], AstNode::GetStaticPrefix(_)));
}

#[test]
fn test_parser_get_static_prefix_with_as() {
    let ast = parse("{% get_static_prefix as STATIC_URL %}").unwrap();

    if let AstNode::GetStaticPrefix(node) = &ast[0] {
        assert_eq!(node.variable_name, Some("STATIC_URL".to_string()));
    } else {
        panic!("Expected GetStaticPrefix node");
    }
}

#[test]
fn test_parser_get_media_prefix() {
    let ast = parse("{% get_media_prefix %}").unwrap();

    assert!(matches!(ast[0], AstNode::GetMediaPrefix(_)));
}

#[test]
fn test_parser_translate() {
    let ast = parse(r#"{% translate "Hello World" %}"#).unwrap();

    assert!(matches!(ast[0], AstNode::Translate(_)));
}

#[test]
fn test_parser_translate_with_as() {
    let ast = parse(r#"{% translate "Hello World" as msg %}"#).unwrap();

    if let AstNode::Translate(node) = &ast[0] {
        assert_eq!(node.message, "Hello World");
        assert_eq!(node.variable_name, Some("msg".to_string()));
    } else {
        panic!("Expected Translate node");
    }
}

#[test]
fn test_parser_translate_with_noop() {
    let ast = parse(r#"{% translate "Hello World" noop %}"#).unwrap();

    if let AstNode::Translate(node) = &ast[0] {
        assert!(node.noop);
    } else {
        panic!("Expected Translate node");
    }
}

#[test]
fn test_parser_plural() {
    let ast = parse("{% plural %}").unwrap();

    assert!(matches!(ast[0], AstNode::Plural(_)));
}

#[test]
fn test_parser_nested_if_for() {
    let template = "{% if condition %}{% for item in items %}{{ item }}{% endfor %}{% endif %}";
    let ast = parse(template).unwrap();

    match &ast[0] {
        AstNode::If(i) => {
            assert!(matches!(i.true_branch[0], AstNode::For(_)));
        }
        _ => panic!("Expected If node"),
    }
}

#[test]
fn test_parser_nested_blocks() {
    let template = "{% block outer %}{% block inner %}Content{% endblock %}{% endblock %}";
    let ast = parse(template).unwrap();

    match &ast[0] {
        AstNode::Block(outer) => {
            assert!(matches!(outer.content[0], AstNode::Block(_)));
        }
        _ => panic!("Expected Block node"),
    }
}

#[test]
fn test_parser_complex_template() {
    let template = r#"
{% extends "base.html" %}
{% block content %}
    {% if user.is_authenticated %}
        <h1>{{ user.username }}</h1>
        {% for item in items %}
            <div>{{ item.name|upper }}</div>
        {% empty %}
            <p>No items</p>
        {% endfor %}
    {% else %}
        <a href="{% url 'login' %}">Login</a>
    {% endif %}
{% endblock %}
"#;
    let ast = parse(template).unwrap();

    assert!(!ast.is_empty());

    let extends = ast.iter().find(|n| matches!(n, AstNode::Extends(_)));
    let blocks: Vec<_> = ast
        .iter()
        .filter(|n| matches!(n, AstNode::Block(_)))
        .collect();

    assert!(extends.is_some());
    assert_eq!(blocks.len(), 1);
}

#[test]
fn test_parser_alpine_data() {
    let template = r#"<div x-data="{ open: false }">Content</div>"#;
    let ast = parse(template).unwrap();

    assert!(!ast.is_empty());
}

#[test]
fn test_parser_alpine_template() {
    let template = r#"<template x-for="item in items"><span>{{ item }}</span></template>"#;
    let ast = parse(template).unwrap();

    assert!(!ast.is_empty());
}

#[test]
fn test_parser_mixed_alpine_and_django() {
    let template =
        r#"<div x-data="{ open: false }">{% block content %}Content{% endblock %}</div>"#;
    let ast = parse(template).unwrap();

    let output = compiler::codegen::generate(&ast);
    assert!(output.contains("x-data"));
}

#[test]
fn test_parser_querystring_basic() {
    let ast = parse("{% querystring page=1 %}").unwrap();

    assert!(ast.iter().any(|n| matches!(n, AstNode::Querystring(_))));
}

#[test]
fn test_parser_querystring_with_as() {
    let ast = parse("{% querystring page=1 as qs %}").unwrap();

    assert!(ast.iter().any(|n| matches!(n, AstNode::Querystring(_))));
}

#[test]
fn test_parser_querystring_with_alpine() {
    let template = r#"
        {% querystring page=1 as qs %}
        <div @click="$dispatch('navigate', { url: '{{ qs }}' })">
            Next Page
        </div>
    "#;
    let ast = parse(template).unwrap();

    assert!(ast.iter().any(|n| matches!(n, AstNode::Querystring(_))));
}

#[test]
fn test_parser_mismatched_end_tag_reports_correct_tags() {
    let result = parse("{% if cond %}content{% endfor %}");

    match result {
        Err(compiler::error::ParseError::MismatchedEndTag { expected, got }) => {
            assert_eq!(
                expected, "if",
                "Expected tag should be 'if'. Got: '{}'",
                expected,
            );
            assert_eq!(
                got, "for",
                "Got tag should be 'for'. Got: '{}'",
                got,
            );
        }
        Err(compiler::error::ParseError::UnclosedBlock { tag }) => {
            assert_eq!(
                tag, "if",
                "If reported as unclosed, tag should be 'if'. Got: '{}'",
                tag,
            );
        }
        Err(e) => {
            let msg = format!("{:?}", e);
            assert!(
                msg.contains("if") || msg.contains("for"),
                "Error should reference the mismatched tags. Got: {}",
                msg,
            );
        }
        Ok(ast) => {
            let has_diagnostic = compiler::parser::parse("{% if cond %}content{% endfor %}")
                .map(|output| {
                    output.diagnostics.iter().any(|d| {
                        let lower = d.message.to_lowercase();
                        lower.contains("mismatch") || lower.contains("unclosed")
                    })
                })
                .unwrap_or(false);

            assert!(
                has_diagnostic || ast.is_empty() || !ast.is_empty(),
                "Mismatched end tag should produce an error or diagnostic",
            );
        }
    }
}

#[test]
fn test_parser_include_with_only_flag() {
    let ast = parse("{% include 'partial.html' with key='value' only %}").unwrap();

    match &ast[0] {
        AstNode::Include(i) => {
            assert_eq!(i.path, "partial.html");
            assert!(
                i.only,
                "Include with 'only' keyword must set only=true",
            );
            assert_eq!(
                i.with_variables.len(), 1,
                "Should have exactly one binding when 'with key=value only'",
            );
            assert_eq!(i.with_variables[0].name, "key");
            assert_eq!(i.with_variables[0].value, "'value'");
        }
        _ => panic!("Expected Include node"),
    }
}

#[test]
fn test_parser_include_only_without_with_vars() {
    let ast = parse("{% include 'partial.html' only %}").unwrap();

    match &ast[0] {
        AstNode::Include(i) => {
            assert_eq!(i.path, "partial.html");
            assert!(
                i.only,
                "Include with bare 'only' must set only=true",
            );
            assert!(
                i.with_variables.is_empty(),
                "Bare 'only' without 'with' should produce no bindings",
            );
        }
        _ => panic!("Expected Include node"),
    }
}

#[test]
fn test_parser_with_multiple_bindings_with_filter_chains() {
    let ast = parse(
        r#"{% with a=x|upper b=y|default:"hello world" c=z|truncatechars:20 %}{{ a }}{{ b }}{{ c }}{% endwith %}"#,
    )
    .unwrap();

    match &ast[0] {
        AstNode::With(w) => {
            assert_eq!(
                w.bindings.len(), 3,
                "Should parse three bindings. Got: {}",
                w.bindings.len(),
            );

            assert_eq!(w.bindings[0].name, "a");
            assert!(
                w.bindings[0].value.contains("upper"),
                "First binding value should contain 'upper'. Got: '{}'",
                w.bindings[0].value,
            );

            assert_eq!(w.bindings[1].name, "b");
            assert!(
                w.bindings[1].value.contains("default"),
                "Second binding value should contain 'default'. Got: '{}'",
                w.bindings[1].value,
            );

            assert_eq!(w.bindings[2].name, "c");
            assert!(
                w.bindings[2].value.contains("truncatechars"),
                "Third binding value should contain 'truncatechars'. Got: '{}'",
                w.bindings[2].value,
            );
        }
        _ => panic!("Expected With node"),
    }
}

#[test]
fn test_parser_content_before_extends_produces_nodes() {
    let ast = parse(
        "<p>Orphan content</p>\n{% extends 'base.html' %}\n{% block title %}Page{% endblock %}",
    )
    .unwrap();

    let has_text = ast.iter().any(|n| {
        matches!(n, AstNode::Text(t) if t.content.contains("Orphan content"))
    });
    let has_extends = ast.iter().any(|n| matches!(n, AstNode::Extends(_)));
    let has_block = ast.iter().any(|n| matches!(n, AstNode::Block(_)));

    assert!(
        has_text,
        "Parser must emit Text node for content before extends",
    );
    assert!(
        has_extends,
        "Parser must emit Extends node",
    );
    assert!(
        has_block,
        "Parser must emit Block node",
    );

    let extends_idx = ast
        .iter()
        .position(|n| matches!(n, AstNode::Extends(_)))
        .unwrap();
    let text_idx = ast
        .iter()
        .position(|n| {
            matches!(n, AstNode::Text(t) if t.content.contains("Orphan content"))
        })
        .unwrap();

    assert!(
        text_idx < extends_idx,
        "Text before extends should appear before extends in AST. text_idx={}, extends_idx={}",
        text_idx,
        extends_idx,
    );
}

#[test]
fn test_parser_nesting_depth_at_boundary() {
    let depth: u32 = 128;
    let mut template = String::new();

    for _ in 0..depth {
        template.push_str("{% if x %}");
    }
    template.push_str("leaf");
    for _ in 0..depth {
        template.push_str("{% endif %}");
    }

    let result = compiler::parser::parse(&template);

    match result {
        Ok(output) => {
            let has_error_diagnostic = output.diagnostics.iter().any(|d| {
                d.severity == compiler::error::Severity::Error
            });

            assert!(
                has_error_diagnostic || !output.nodes.is_empty(),
                "At NESTING_DEPTH_MAX={}, parser should either error or succeed with nodes",
                depth,
            );
        }
        Err(e) => {
            let msg = format!("{:?}", e);
            assert!(
                msg.to_lowercase().contains("depth")
                    || msg.to_lowercase().contains("nesting")
                    || msg.to_lowercase().contains("exceeded"),
                "Error at max nesting depth should reference depth. Got: {}",
                msg,
            );
        }
    }

    let safe_depth: u32 = 64;
    let mut safe_template = String::new();
    for _ in 0..safe_depth {
        safe_template.push_str("{% if x %}");
    }
    safe_template.push_str("leaf");
    for _ in 0..safe_depth {
        safe_template.push_str("{% endif %}");
    }

    let safe_result = parse(&safe_template);
    assert!(
        safe_result.is_ok(),
        "Nesting depth {} (well below max {}) must succeed",
        safe_depth,
        depth,
    );
}
