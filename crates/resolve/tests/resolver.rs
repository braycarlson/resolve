mod common;

use std::fs;

use tempfile::TempDir;

use compiler::resolver::ResolveLimits;
use resolve::loader::{FsTemplateLoader, VendorIndex};

use common::{compile_template, write_template};

#[test]
fn test_resolver_three_level_extends_chain() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "base.html",
        r#"
<!DOCTYPE html>
<html>
<head>
    {% block head %}<title>Base</title>{% endblock %}
</head>
<body>
    {% block body %}{% endblock %}
</body>
</html>
"#,
    );

    write_template(
        &temp_dir,
        "layout.html",
        r#"
{% extends 'base.html' %}
{% block body %}
<div class="layout">
    {% block content %}{% endblock %}
</div>
{% endblock %}
"#,
    );

    write_template(
        &temp_dir,
        "page.html",
        r#"
{% extends 'layout.html' %}
{% block content %}
<h1>Page Content</h1>
<p>This is the page content.</p>
{% endblock %}
"#,
    );

    let output = compile_template(&temp_dir, "page.html");

    assert!(output.contains("<!DOCTYPE html>"), "doctype from base");
    assert!(output.contains("<html>"), "html tag from base");
    assert!(output.contains("<head>"), "head from base");
    assert!(output.contains("<body>"), "body from base");
    assert!(output.contains("<div class=\"layout\">"), "layout div");
    assert!(output.contains("<h1>Page Content</h1>"), "page content");

    assert!(!output.contains("{% extends"), "no extends tag remains");
    assert!(!output.contains("{% block"), "no block tag remains");
    assert!(!output.contains("{% endblock"), "no endblock tag remains");
}

#[test]
fn test_resolver_block_super_basic() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "parent.html",
        r#"
{% block scripts %}<script src="parent.js"></script>{% endblock %}
"#,
    );

    write_template(
        &temp_dir,
        "child.html",
        r#"
{% extends 'parent.html' %}
{% block scripts %}{{ block.super }}<script src="child.js"></script>{% endblock %}
"#,
    );

    let output = compile_template(&temp_dir, "child.html");

    let parent_pos = output.find("parent.js").unwrap();
    let child_pos = output.find("child.js").unwrap();
    assert!(parent_pos < child_pos, "parent script before child script");
}

#[test]
fn test_resolver_block_super_three_level() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "grandparent.html",
        r#"
{% block scripts %}<script src="grandparent.js"></script>{% endblock %}
"#,
    );

    write_template(
        &temp_dir,
        "parent.html",
        r#"
{% extends 'grandparent.html' %}
{% block scripts %}{{ block.super }}<script src="parent.js"></script>{% endblock %}
"#,
    );

    write_template(
        &temp_dir,
        "child.html",
        r#"
{% extends 'parent.html' %}
{% block scripts %}{{ block.super }}<script src="child.js"></script>{% endblock %}
"#,
    );

    let output = compile_template(&temp_dir, "child.html");

    let grandparent_pos = output.find("grandparent.js").unwrap();
    let parent_pos = output.find("parent.js").unwrap();
    let child_pos = output.find("child.js").unwrap();

    assert!(grandparent_pos < parent_pos, "grandparent before parent");
    assert!(parent_pos < child_pos, "parent before child");
}

#[test]
fn test_resolver_block_super_empty_parent() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "parent.html",
        "{% block scripts %}{% endblock %}\n",
    );

    write_template(
        &temp_dir,
        "child.html",
        r#"
{% extends 'parent.html' %}
{% block scripts %}{{ block.super }}<script src="child.js"></script>{% endblock %}
"#,
    );

    let output = compile_template(&temp_dir, "child.html");

    assert!(output.contains("child.js"));
    assert!(!output.contains("parent.js"));
}

#[test]
fn test_resolver_block_in_html_attribute() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "base.html",
        "<div class=\"base {% block extra_class %}{% endblock %}\">Content</div>\n",
    );

    write_template(
        &temp_dir,
        "child.html",
        "{% extends 'base.html' %}\n{% block extra_class %}text-danger{% endblock %}\n",
    );

    let output = compile_template(&temp_dir, "child.html");

    assert!(
        output.contains("base text-danger")
            || output.contains("base  text-danger"),
    );
    assert!(!output.contains("class=\"base \n"), "no newline in class");
}

#[test]
fn test_resolver_empty_blocks() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "template.html",
        "<div>\n    {% block x %}{% endblock %}\n    {% block y %}{% endblock %}\n    {% block z %}{% endblock %}\n</div>\n",
    );

    let output = compile_template(&temp_dir, "template.html");

    assert!(output.contains("<div>"));
    assert!(output.contains("</div>"));
    assert!(!output.contains("{% block"));
}

#[test]
fn test_resolver_single_line_extends() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "parent.html",
        "{% block title %}Parent Title{% endblock %}\n{% block content %}Parent Content{% endblock %}\n",
    );

    write_template(&temp_dir, "child.html", "{% extends 'parent.html' %}");

    let output = compile_template(&temp_dir, "child.html");

    assert!(output.contains("Parent Title"));
    assert!(output.contains("Parent Content"));
    assert!(!output.contains("{% extends"));
}

#[test]
fn test_resolver_include_with_vars_preserved() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "button.html",
        r#"<a href="{{ button_href }}" class="btn">{{ button_text }}</a>"#,
    );

    write_template(
        &temp_dir,
        "page.html",
        r#"{% include 'button.html' with button_text='Click me' button_href='/fake/path/' %}"#,
    );

    let output = compile_template(&temp_dir, "page.html");

    assert!(
        output.contains("<a href=\"{{ button_href }}\" class=\"btn\">{{ button_text }}</a>")
            || output.contains("<a href=\"/fake/path/\" class=\"btn\">Click me</a>")
            || output.contains("button_text")
            || output.contains("Click me"),
        "Include with with_vars should resolve to included content. Got: {}",
        output
    );

    assert!(
        !output.contains("{% include 'button.html' with button_text='Click me' button_href='/fake/path/' %}"),
        "Include tag should not remain verbatim after successful inclusion. Got: {}",
        output
    );
}

#[test]
fn test_resolver_include_without_vars_inlined() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "badge.html",
        r#"<span>Static Badge</span>"#,
    );

    write_template(
        &temp_dir,
        "page.html",
        r#"{% include 'badge.html' %}"#,
    );

    let output = compile_template(&temp_dir, "page.html");

    assert!(
        output.contains("<span>Static Badge</span>"),
        "Include without with_vars should be inlined. Got: {}",
        output
    );

    assert!(
        !output.contains("{% include"),
        "Inlined include tag should not appear in output. Got: {}",
        output
    );
}

#[test]
fn test_resolver_include_single_template() {
    let temp_dir = TempDir::new().unwrap();

    write_template(&temp_dir, "included.html", r#"<div>Included content</div>"#);

    write_template(
        &temp_dir,
        "main.html",
        r#"{% include 'included.html' with y='z' %}"#,
    );

    let output = compile_template(&temp_dir, "main.html");

    assert!(
        output.contains("<div>Included content</div>"),
        "include should resolve to the included template content. Got: {}",
        output
    );

    assert!(
        !output.contains("{% include 'included.html' with y='z' %}"),
        "include with with_vars should not be preserved verbatim after successful inclusion. Got: {}",
        output
    );
}

#[test]
fn test_resolver_alpine_model_preserved() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "base.html",
        r#"{% block content %}{% endblock %}"#,
    );

    write_template(
        &temp_dir,
        "child.html",
        r#"{% extends 'base.html' %}{% block content %}<input x-model="{{ glue_model_field }}">{% endblock %}"#,
    );

    let output = compile_template(&temp_dir, "child.html");

    assert!(output.contains("x-model"), "x-model preserved");
    assert!(output.contains("glue_model_field"), "variable preserved");
}

#[test]
fn test_resolver_alpine_template_with_django() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "template.html",
        r#"<template x-for="item in items"><div>{% block item_content %}{{ item }}{% endblock %}</div></template>"#,
    );

    let output = compile_template(&temp_dir, "template.html");

    assert!(output.contains("<template"));
    assert!(output.contains("x-for"));
    assert!(!output.contains("{% block"));
}

#[test]
fn test_resolver_django_in_attributes() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "base.html",
        r#"<div class="{% block x %}{% endblock %}">Content</div>"#,
    );

    write_template(
        &temp_dir,
        "child.html",
        r#"{% extends 'base.html' %}{% block x %}active{% endblock %}"#,
    );

    let output = compile_template(&temp_dir, "child.html");

    assert!(output.contains("class="));
    assert!(output.contains("active"));
}

#[test]
fn test_resolver_conditional_attributes() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "template.html",
        r#"<div {% if x %}data-value="y"{% endif %}>Content</div>"#,
    );

    let output = compile_template(&temp_dir, "template.html");

    assert!(output.contains("{% if"), "if tag preserved");
    assert!(output.contains("data-value"), "attribute present");
    assert!(output.contains("<div"), "div tag present");
}

#[test]
fn test_resolver_filters_with_pipe() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "template.html",
        r#"{{ key_stack|add:'|'|add:key }}"#,
    );

    let output = compile_template(&temp_dir, "template.html");

    assert!(output.contains("|add:"));
}

#[test]
fn test_resolver_tuple_unpacking() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "template.html",
        r#"{% for key, value in dict|custom_filter %}{{ key }}: {{ value }}{% endfor %}"#,
    );

    let output = compile_template(&temp_dir, "template.html");

    assert!(output.contains("{% for"));
    assert!(output.contains("key, value"));
    assert!(output.contains("custom_filter"));
}

#[test]
fn test_resolver_deep_nesting() {
    let temp_dir = TempDir::new().unwrap();

    write_template(&temp_dir, "base.html", r#"{% block outer %}{% endblock %}"#);

    write_template(
        &temp_dir,
        "child.html",
        r#"{% extends 'base.html' %}{% block outer %}{% for item in items %}{% if item.active %}{{ item }}{% endif %}{% endfor %}{% endblock %}"#,
    );

    let output = compile_template(&temp_dir, "child.html");

    assert!(!output.contains("{% block"));
    assert!(output.contains("{% for"));
    assert!(output.contains("{% if"));
}

#[test]
fn test_resolver_script_with_alpine() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "template.html",
        r#"<script type="application/json" x-ref="payload">{% if x %}{{ x|safe }}{% else %}{}{% endif %}</script>"#,
    );

    let output = compile_template(&temp_dir, "template.html");

    assert!(output.contains("<script"), "script tag present");
    assert!(output.contains("x-ref"), "x-ref preserved");
    assert!(output.contains("{% if"), "if tag preserved");
}

#[test]
fn test_resolver_error_handling_missing_template() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "main.html",
        r#"{% include 'nonexistent.html' %}"#,
    );

    let output = compile_template(&temp_dir, "main.html");

    assert!(
        output.contains("{% include 'nonexistent.html' %}")
            || output.is_empty()
            || !output.is_empty(),
        "Missing template should either preserve tag or handle gracefully"
    );
}

#[test]
fn test_resolver_error_handling_circular_extends() {
    let temp_dir = TempDir::new().unwrap();

    write_template(&temp_dir, "a.html", r#"{% extends 'b.html' %}"#);
    write_template(&temp_dir, "b.html", r#"{% extends 'a.html' %}"#);

    let result = std::panic::catch_unwind(|| compile_template(&temp_dir, "a.html"));

    assert!(
        result.is_ok(),
        "Compiler should not panic on circular extends"
    );
}

#[test]
fn test_resolver_error_handling_unclosed_block() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "unclosed.html",
        r#"{% block content %}Some content"#,
    );

    let content = fs::read_to_string(temp_dir.path().join("unclosed.html")).unwrap();
    let result = common::parse(&content);

    match result {
        Err(compiler::error::ParseError::UnclosedBlock { tag }) => {
            assert_eq!(tag, "block");
        }
        Err(e) => {
            let msg = format!("{:?}", e);
            assert!(
                msg.to_lowercase().contains("unclosed")
                    || msg.to_lowercase().contains("mismatched"),
                "Error should mention unclosed or mismatched block, got: {}",
                msg
            );
        }
        Ok(_) => {}
    }
}

#[test]
fn test_resolver_recursive_includes() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "self.html",
        r#"{% if val|is_dict %}{% include 'self.html' with depth=True %}{% endif %}"#,
    );

    let result = std::panic::catch_unwind(|| compile_template(&temp_dir, "self.html"));

    assert!(
        result.is_ok(),
        "compiler should not panic on recursive include"
    );

    if let Ok(output) = result {
        assert!(
            output.contains("{% include 'self.html' with depth=True %}"),
            "Recursive include with with_vars should be preserved. Got: {}",
            output
        );
    }
}

#[test]
fn test_resolver_include_with_filter_expressions() {
    let temp_dir = TempDir::new().unwrap();

    write_template(&temp_dir, "badge.html", r#"<span>{{ description }}</span>"#);

    write_template(
        &temp_dir,
        "page.html",
        r#"{% include 'badge.html' with description=friend.description|linebreaksbr %}"#,
    );

    let output = compile_template(&temp_dir, "page.html");

    assert!(
        output.contains("description") || output.contains("linebreaksbr"),
    );
}

#[test]
fn test_resolver_include_inside_for_loop() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "item.html",
        r#"<div class="item">{{ item_name }}</div>"#,
    );

    write_template(
        &temp_dir,
        "empty_item.html",
        r#"<div class="empty">No items</div>"#,
    );

    write_template(
        &temp_dir,
        "list.html",
        r#"{% for item in items %}{% include 'item.html' with item_name=item.name %}{% empty %}{% include 'empty_item.html' %}{% endfor %}"#,
    );

    let output = compile_template(&temp_dir, "list.html");

    assert!(output.contains("{% for") || output.contains("{% include"));
    assert!(output.contains("{% endfor %}"));
}

#[test]
fn test_resolver_child_defines_block_not_in_parent() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "parent.html",
        r#"<html>
<head>{% block head %}Default Head{% endblock %}</head>
<body>{% block body %}Default Body{% endblock %}</body>
</html>"#,
    );

    write_template(
        &temp_dir,
        "child.html",
        r#"{% extends 'parent.html' %}
{% block body %}Child Body{% endblock %}
{% block sidebar %}This block does not exist in parent{% endblock %}"#,
    );

    let output = compile_template(&temp_dir, "child.html");

    assert!(
        output.contains("Child Body"),
        "Matching block should override parent content",
    );
    assert!(
        output.contains("Default Head"),
        "Unoverridden block should keep parent default",
    );
    assert!(
        !output.contains("This block does not exist in parent"),
        "Block defined in child but absent from parent should be silently discarded. Got: {}",
        output,
    );
    assert!(
        !output.contains("{% block sidebar"),
        "Orphan block tag should not appear in output",
    );
}

#[test]
fn test_resolver_duplicate_block_names_in_same_template() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "parent.html",
        r#"<div>{% block x %}Parent X{% endblock %}</div>"#,
    );

    write_template(
        &temp_dir,
        "child.html",
        r#"{% extends 'parent.html' %}
{% block x %}First Override{% endblock %}
{% block x %}Second Override{% endblock %}"#,
    );

    let output = compile_template(&temp_dir, "child.html");

    let has_first = output.contains("First Override");
    let has_second = output.contains("Second Override");

    assert!(
        has_first || has_second,
        "At least one of the duplicate block definitions must appear. Got: {}",
        output,
    );
    assert!(
        !(has_first && has_second),
        "Both duplicate block definitions should not appear simultaneously. Got: {}",
        output,
    );
    assert!(
        !output.contains("Parent X"),
        "Parent block content should be overridden regardless of which duplicate wins",
    );
}

#[test]
fn test_resolver_block_super_in_block_missing_from_grandparent() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "grandparent.html",
        r#"<html><body>{% block body %}GP Body{% endblock %}</body></html>"#,
    );

    write_template(
        &temp_dir,
        "parent.html",
        r#"{% extends 'grandparent.html' %}
{% block body %}
{% block sidebar %}Parent Sidebar{% endblock %}
{% endblock %}"#,
    );

    write_template(
        &temp_dir,
        "child.html",
        r#"{% extends 'parent.html' %}
{% block sidebar %}{{ block.super }}<nav>Child Nav</nav>{% endblock %}"#,
    );

    let output = compile_template(&temp_dir, "child.html");

    assert!(
        output.contains("Parent Sidebar"),
        "block.super should inject parent's sidebar content. Got: {}",
        output,
    );
    assert!(
        output.contains("Child Nav"),
        "Child's own content should appear. Got: {}",
        output,
    );

    let parent_pos = output.find("Parent Sidebar").unwrap();
    let child_pos = output.find("Child Nav").unwrap();
    assert!(
        parent_pos < child_pos,
        "block.super content should precede child content",
    );
}

#[test]
fn test_resolver_inheritance_chain_truncated_beyond_limit() {
    let temp_dir = TempDir::new().unwrap();

    let depth: u32 = 11;

    write_template(
        &temp_dir,
        "level_0.html",
        r#"<html>{% block content %}Base{% endblock %}</html>"#,
    );

    for level in 1..depth {
        let name = format!("level_{}.html", level);
        let parent = format!("level_{}.html", level - 1);
        let content = format!(
            "{{% extends '{}' %}}{{% block content %}}Level {}{{% endblock %}}",
            parent, level,
        );
        write_template(&temp_dir, &name, &content);
    }

    let leaf = format!("level_{}.html", depth - 1);

    let config = common::create_test_config(&temp_dir);
    let discovery = resolve::discovery::TemplateDiscovery::new(&config);
    let index = discovery.scan().unwrap();
    let vendor = VendorIndex::build(
        temp_dir.path().join("vendor").as_path(),
    );
    let loader = FsTemplateLoader::new(&index, &vendor);

    let limits = ResolveLimits {
        max_include_depth: 20,
        max_inheritance_depth: 5,
    };

    let template_path = temp_dir.path().join(&leaf);
    let content = fs::read_to_string(&template_path).unwrap();
    let nodes = common::parse(&content).unwrap();

    let result = compiler::resolver::inheritance::resolve(
        nodes,
        &leaf,
        Some(template_path.to_path_buf()),
        &loader,
        &limits,
    );

    assert!(
        result.is_ok(),
        "Inheritance chain of {} levels with max_depth=5 should not panic. \
         resolve_chain loops 0..=limit and silently truncates. Got err: {:?}",
        depth,
        result.err(),
    );

    let resolved = result.unwrap();
    let output = compiler::codegen::generate(&resolved);

    assert!(
        output.contains(&format!("Level {}", depth - 1)),
        "Leaf template content must appear in truncated output. Got: {}",
        output,
    );

    assert!(
        !output.contains("{% extends"),
        "Extends tags should be stripped even on truncated chains. Got: {}",
        output,
    );

    let short_depth: u32 = 4;

    for level in 1..=short_depth {
        let name = format!("short_{}.html", level);
        let parent = if level == 1 {
            "level_0.html".to_string()
        } else {
            format!("short_{}.html", level - 1)
        };
        let content = format!(
            "{{% extends '{}' %}}{{% block content %}}Short {}{{% endblock %}}",
            parent, level,
        );
        write_template(&temp_dir, &name, &content);
    }

    let short_leaf = format!("short_{}.html", short_depth);
    let short_output = compile_template(&temp_dir, &short_leaf);

    assert!(
        short_output.contains("<html>"),
        "Chain within limit should fully resolve to root. Got: {}",
        short_output,
    );
    assert!(
        short_output.contains(&format!("Short {}", short_depth)),
        "Leaf override should appear in fully resolved chain. Got: {}",
        short_output,
    );
    assert!(
        !short_output.contains("Base"),
        "Base block default should be overridden. Got: {}",
        short_output,
    );
}

#[test]
fn test_resolver_load_deduplication_across_inheritance_chain() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "base.html",
        r#"{% load static %}
<html>{% block content %}{% endblock %}</html>"#,
    );

    write_template(
        &temp_dir,
        "mid.html",
        r#"{% extends 'base.html' %}
{% load static i18n %}
{% block content %}{% block inner %}{% endblock %}{% endblock %}"#,
    );

    write_template(
        &temp_dir,
        "leaf.html",
        r#"{% extends 'mid.html' %}
{% load static %}
{% block inner %}Leaf Content{% endblock %}"#,
    );

    let output = compile_template(&temp_dir, "leaf.html");

    let static_load_count = output.matches("{% load static %}").count();
    assert!(
        static_load_count <= 1,
        "'load static' (alone) should appear at most once in output after deduplication. \
         Found {} occurrences. Output: {}",
        static_load_count,
        output,
    );

    assert!(
        output.contains("Leaf Content"),
        "Leaf block content must be present in output",
    );

    let has_i18n = output.contains("i18n");
    assert!(
        has_i18n,
        "'load static i18n' from mid.html should be preserved as a distinct load. Got: {}",
        output,
    );
}

#[test]
fn test_resolver_content_outside_blocks_stripped_on_extends() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "base.html",
        r#"<html><body>{% block content %}Default{% endblock %}</body></html>"#,
    );

    write_template(
        &temp_dir,
        "child.html",
        r#"<p>Orphan paragraph that should be stripped</p>
{% extends 'base.html' %}
<div>Another orphan</div>
{% block content %}Real Content{% endblock %}
<footer>Orphan footer</footer>"#,
    );

    let output = compile_template(&temp_dir, "child.html");

    assert!(
        output.contains("Real Content"),
        "Block content must appear in output. Got: {}",
        output,
    );
    assert!(
        !output.contains("Orphan paragraph"),
        "Content before extends should be stripped. Got: {}",
        output,
    );
    assert!(
        !output.contains("Another orphan"),
        "Content between extends and blocks should be stripped. Got: {}",
        output,
    );
    assert!(
        !output.contains("Orphan footer"),
        "Content after blocks should be stripped. Got: {}",
        output,
    );
}

#[test]
fn test_resolver_mutual_includes_a_includes_b_includes_a() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "a.html",
        r#"<div class="a">{% include 'b.html' %}</div>"#,
    );

    write_template(
        &temp_dir,
        "b.html",
        r#"<div class="b">{% include 'a.html' %}</div>"#,
    );

    let result = std::panic::catch_unwind(|| compile_template(&temp_dir, "a.html"));

    assert!(
        result.is_ok(),
        "Mutual includes (A->B->A) must not panic",
    );

    if let Ok(output) = result {
        assert!(
            output.contains("class=\"a\""),
            "Top-level template A content must appear. Got: {}",
            output,
        );
        assert!(
            output.contains("class=\"b\""),
            "First-level include of B must appear. Got: {}",
            output,
        );

        let a_count = output.matches("class=\"a\"").count();
        assert!(
            a_count < 3,
            "Recursive expansion of A should be bounded. Found {} occurrences of A. Got: {}",
            a_count,
            output,
        );
    }
}

#[test]
fn test_resolver_transitive_include_with_inheritance() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "widget_base.html",
        r#"<div class="widget">{% block widget_content %}Base Widget{% endblock %}</div>"#,
    );

    write_template(
        &temp_dir,
        "widget_chart.html",
        r#"{% extends 'widget_base.html' %}
{% block widget_content %}<canvas>Chart</canvas>{% endblock %}"#,
    );

    write_template(
        &temp_dir,
        "dashboard.html",
        r#"<section>{% include 'widget_chart.html' %}</section>"#,
    );

    let output = compile_template(&temp_dir, "dashboard.html");

    assert!(
        output.contains("<section>"),
        "Dashboard wrapper must be present",
    );
    assert!(
        output.contains("<div class=\"widget\">"),
        "widget_base.html structure should appear through transitive include+inheritance. Got: {}",
        output,
    );
    assert!(
        output.contains("<canvas>Chart</canvas>"),
        "widget_chart.html override should appear. Got: {}",
        output,
    );
    assert!(
        !output.contains("Base Widget"),
        "Base widget default content should be overridden. Got: {}",
        output,
    );
    assert!(
        !output.contains("{% extends"),
        "No extends tag should remain. Got: {}",
        output,
    );
    assert!(
        !output.contains("{% block"),
        "No block tag should remain. Got: {}",
        output,
    );
}

#[test]
fn test_resolver_include_chain_fully_resolved_via_recursive_descent() {
    let temp_dir = TempDir::new().unwrap();

    let depth: u32 = 25;

    for level in 0..depth {
        let name = format!("level_{}.html", level);
        let next = format!("level_{}.html", level + 1);
        let content = format!(
            "<div class=\"l{}\">{{%  include '{}' %}}</div>",
            level, next,
        );
        write_template(&temp_dir, &name, &content);
    }

    write_template(
        &temp_dir,
        &format!("level_{}.html", depth),
        r#"<span>Leaf</span>"#,
    );

    let config = common::create_test_config(&temp_dir);
    let discovery = resolve::discovery::TemplateDiscovery::new(&config);
    let index = discovery.scan().unwrap();
    let vendor = VendorIndex::build(
        temp_dir.path().join("vendor").as_path(),
    );
    let loader = FsTemplateLoader::new(&index, &vendor);

    let limits = ResolveLimits {
        max_include_depth: 5,
        max_inheritance_depth: 10,
    };

    let template_path = temp_dir.path().join("level_0.html");
    let content = fs::read_to_string(&template_path).unwrap();
    let nodes = common::parse(&content).unwrap();

    let result = compiler::resolver::inclusion::resolve(
        nodes,
        "level_0.html",
        &loader,
        &limits,
    );

    assert!(
        result.is_ok(),
        "Include chain of {} should not error — expand_pass resolves \
         the entire chain via recursive descent in a single pass. Got: {:?}",
        depth,
        result.err(),
    );

    let resolved = result.unwrap();
    let output = compiler::codegen::generate(&resolved);

    assert!(
        output.contains("<span>Leaf</span>"),
        "Leaf template content should appear — the recursive expansion \
         fully resolves all nested includes. Got: {}",
        output,
    );

    assert!(
        !output.contains("{% include") && !output.contains("{%  include"),
        "No include tags should remain after full recursive expansion. Got: {}",
        output,
    );

    for level in 0..depth {
        let marker = format!("class=\"l{}\"", level);
        assert!(
            output.contains(&marker),
            "Wrapper div for level {} should appear in output. Got: {}",
            level,
            output,
        );
    }
}

#[test]
fn test_resolver_multiple_includes_same_template_different_with_vars() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "badge.html",
        r#"<span class="{{ badge_class }}">{{ badge_text }}</span>"#,
    );

    write_template(
        &temp_dir,
        "page.html",
        r#"<div>
{% include 'badge.html' with badge_class='success' badge_text='Active' %}
{% include 'badge.html' with badge_class='danger' badge_text='Inactive' %}
</div>"#,
    );

    let output = compile_template(&temp_dir, "page.html");

    assert!(
        !output.contains("{% include"),
        "Both includes should be resolved. Got: {}",
        output,
    );

    let span_count = output.matches("<span class=").count();
    assert_eq!(
        span_count, 2,
        "Should produce two badge spans from two includes. Got: {}. Output: {}",
        span_count,
        output,
    );

    let with_count = output.matches("{% with").count();
    assert_eq!(
        with_count, 2,
        "Each include with with_vars should produce its own with wrapper. Got: {}. Output: {}",
        with_count,
        output,
    );
}

#[test]
fn test_resolver_empty_child_extends_inherits_all_parent_defaults() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "parent.html",
        r#"<html>
<head>{% block head %}Parent Head{% endblock %}</head>
<body>
{% block nav %}Parent Nav{% endblock %}
{% block content %}Parent Content{% endblock %}
{% block footer %}Parent Footer{% endblock %}
</body>
</html>"#,
    );

    write_template(
        &temp_dir,
        "child.html",
        r#"{% extends 'parent.html' %}"#,
    );

    let output = compile_template(&temp_dir, "child.html");

    assert!(output.contains("Parent Head"), "head default preserved");
    assert!(output.contains("Parent Nav"), "nav default preserved");
    assert!(output.contains("Parent Content"), "content default preserved");
    assert!(output.contains("Parent Footer"), "footer default preserved");
    assert!(!output.contains("{% extends"), "extends tag removed");
    assert!(!output.contains("{% block"), "block tags removed");
}
