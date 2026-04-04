mod common;

use std::time::Instant;

use tempfile::TempDir;

use compiler::ast::AstNode;

use common::{compile_template, count_nodes_of_type, parse, write_template, NodeType};


#[test]
fn test_integration_deep_inheritance_with_blocks() {
    let template = r"
{% extends 'base.html' %}
{% load static %}

{% block title %}{{ page_title }}{% endblock %}

{% block base_body_content %}
<div class='container'>
    {% for section in sections %}
        <section id='{{ section.id }}'>
            <h2>{{ section.title }}</h2>
            {% if section.items %}
                {% for item in section.items %}
                    {% if item.active %}
                        {% include 'item/active_item.html' with item=item %}
                    {% else %}
                        {% include 'item/inactive_item.html' with item=item %}
                    {% endif %}
                {% endfor %}
            {% else %}
                {% include 'django_spire/item/no_data_item.html' %}
            {% endif %}
        </section>
    {% endfor %}
</div>
{% endblock %}
";

    let ast = parse(template).unwrap();

    assert!(ast.iter().any(|n| matches!(n, AstNode::Extends(_))));
    assert!(ast.iter().any(|n| matches!(n, AstNode::Load(_))));
    assert!(ast.iter().any(|n| matches!(n, AstNode::Block(_))));
    assert!(count_nodes_of_type(&ast, NodeType::For) > 0);
    assert!(count_nodes_of_type(&ast, NodeType::If) > 0);
    assert!(count_nodes_of_type(&ast, NodeType::Include) > 0);
}

#[test]
fn test_integration_complex_if_elif_else_chains() {
    let template = r"
{% if user.is_authenticated %}
    Welcome, {{ user.username }}!
    {% if user.is_staff %}
        {% include 'admin/admin_panel.html' %}
    {% elif user.is_superuser %}
        {% include 'admin/superadmin_panel.html' %}
    {% else %}
        {% include 'user/user_panel.html' %}
    {% endif %}
{% elif user.is_active %}
    {% include 'auth/login_prompt.html' %}
{% else %}
    {% include 'auth/inactive_message.html' %}
{% endif %}
";

    let ast = parse(template).unwrap();

    fn count_all_if_nodes(nodes: &[AstNode]) -> usize {
        let mut count: usize = 0;
        for node in nodes {
            if let AstNode::If(if_node) = node {
                count += 1;
                count += count_all_if_nodes(&if_node.true_branch);
                for elif in &if_node.elif_branches {
                    count += count_all_if_nodes(&elif.body);
                }
                if let Some(else_branch) = &if_node.else_branch {
                    count += count_all_if_nodes(else_branch);
                }
            } else if let AstNode::For(for_node) = node {
                count += count_all_if_nodes(&for_node.body);
                if let Some(empty) = &for_node.empty_branch {
                    count += count_all_if_nodes(empty);
                }
            } else if let AstNode::Block(block_node) = node {
                count += count_all_if_nodes(&block_node.content);
            }
        }
        count
    }

    let total_if_count = count_all_if_nodes(&ast);
    assert!(
        total_if_count >= 2,
        "Expected at least 2 if nodes, found {}",
        total_if_count
    );

    assert!(ast.iter().any(|n| {
        if let AstNode::If(if_node) = n {
            !if_node.elif_branches.is_empty()
        } else {
            false
        }
    }));
}

#[test]
fn test_integration_for_loop_tuple_unpacking() {
    let template = r"
{% for key, value in dict.items %}
    <div class='item'>
        <span class='key'>{{ key }}</span>
        <span class='value'>{{ value }}</span>
    </div>
{% empty %}
    <p>No items</p>
{% endfor %}
";

    let ast = parse(template).unwrap();

    assert!(ast.iter().any(|n| {
        if let AstNode::For(for_node) = n {
            for_node.variable.contains(",")
        } else {
            false
        }
    }));

    assert!(ast.iter().any(|n| {
        if let AstNode::For(for_node) = n {
            for_node.empty_branch.is_some()
        } else {
            false
        }
    }));
}

#[test]
fn test_integration_nested_loops_and_conditionals() {
    let template = r"
{% for section in sections %}
    <section>
        <h2>{{ section.title }}</h2>
        {% for item in section.items %}
            {% if item.visible %}
                {% if item.priority > 5 %}
                    {% include 'item/high_priority.html' with item=item %}
                {% elif item.priority > 3 %}
                    {% include 'item/medium_priority.html' with item=item %}
                {% else %}
                    {% include 'item/low_priority.html' with item=item %}
                {% endif %}
            {% endif %}
        {% endfor %}
    </section>
{% endfor %}
";

    let ast = parse(template).unwrap();

    assert!(count_nodes_of_type(&ast, NodeType::For) >= 2);

    fn has_if_in_for(nodes: &[AstNode]) -> bool {
        for node in nodes {
            if let AstNode::For(for_node) = node {
                if for_node.body.iter().any(|b| matches!(b, AstNode::If(_))) {
                    return true;
                }
                if has_if_in_for(&for_node.body) {
                    return true;
                }
            } else if let AstNode::If(if_node) = node {
                if has_if_in_for(&if_node.true_branch) {
                    return true;
                }
                for elif in &if_node.elif_branches {
                    if has_if_in_for(&elif.body) {
                        return true;
                    }
                }
                if let Some(else_branch) = &if_node.else_branch {
                    if has_if_in_for(else_branch) {
                        return true;
                    }
                }
            }
        }
        false
    }
    assert!(has_if_in_for(&ast), "Expected if inside for loop");
}

#[test]
fn test_integration_compile_performance_is_acceptable() {
    let temp_dir = TempDir::new().unwrap();
    let start = Instant::now();

    write_template(
        &temp_dir,
        "base.html",
        r#"<!DOCTYPE html>
<html>
<head><title>{% block title %}Base{% endblock %}</title></head>
<body>{% block content %}{% endblock %}</body>
</html>"#,
    );

    write_template(
        &temp_dir,
        "layout.html",
        r#"{% extends 'base.html' %}
{% block content %}
<div class="layout">
    {% block inner %}{% endblock %}
</div>
{% endblock %}"#,
    );

    write_template(
        &temp_dir,
        "level2.html",
        r#"{% extends 'layout.html' %}
{% block inner %}
<div class="level2">
    {% block level2_content %}{% endblock %}
</div>
{% endblock %}"#,
    );

    for i in 0..50 {
        write_template(
            &temp_dir,
            &format!("comp_{}.html", i),
            &format!(
                r#"<div class="component-{}">Component {} content</div>"#,
                i, i
            ),
        );
    }

    for page_num in 0..100 {
        let includes: String = (0..20)
            .map(|i| {
                format!(
                    "{{% include 'comp_{}.html' %}}",
                    (page_num + i) % 50
                )
            })
            .collect();

        write_template(
            &temp_dir,
            &format!("pages/page_{}.html", page_num),
            &format!(
                r#"{{% extends 'level2.html' %}}
{{% block level2_content %}}
<h1>Page {}</h1>
{}
{{% endblock %}}"#,
                page_num, includes
            ),
        );
    }

    let mut compiled_count: usize = 0;
    let mut total_output_len: usize = 0;

    for page_num in 0..100 {
        let output = compile_template(
            &temp_dir,
            &format!("pages/page_{}.html", page_num),
        );

        assert!(!output.is_empty(), "page_{}.html should produce output", page_num);
        assert!(
            output.contains("<!DOCTYPE html>"),
            "page_{}.html should contain base content",
            page_num
        );
        assert!(
            output.contains(&format!("<h1>Page {}</h1>", page_num)),
            "page_{}.html should contain page heading",
            page_num
        );
        assert!(
            !output.contains("{% extends"),
            "page_{}.html should have no extends tags",
            page_num
        );
        assert!(
            !output.contains("{% block"),
            "page_{}.html should have no block tags",
            page_num
        );

        compiled_count += 1;
        total_output_len += output.len();
    }

    let duration = start.elapsed();

    assert_eq!(compiled_count, 100, "Should compile all 100 pages");
    assert!(total_output_len > 0, "Total output should be non-empty");
    assert!(
        duration.as_secs() < 30,
        "Should compile 100 templates in under 30 seconds, took {:?}",
        duration
    );
}
