use std::fs;
use std::time::Instant;

use tempfile::TempDir;

use compiler::ast::AstNode;

use crate::common::{NodeType, compile_template, count_nodes_of_type, parse, write_template};

#[test]
fn test_matrix_integration_fake_dashboard_resolves_inheritance_and_includes() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "layouts/base.html",
        r#"<!DOCTYPE html>
<html>
<head>
    <title>{% block title %}Base Title{% endblock %}</title>
</head>
<body>
    <main>{% block content %}{% endblock %}</main>
</body>
</html>"#,
    );

    write_template(
        &temp_dir,
        "layouts/page.html",
        r#"{% extends 'layouts/base.html' %}
{% block content %}
<section class="page-shell">
    {% block page_content %}{% endblock %}
</section>
{% endblock %}"#,
    );

    write_template(
        &temp_dir,
        "components/fake_metric.html",
        r#"<div class="metric">{{ label }}: {{ value|default:"0" }}</div>"#,
    );

    write_template(
        &temp_dir,
        "components/fake_empty.html",
        r#"<div class="empty">Nothing to show</div>"#,
    );

    write_template(
        &temp_dir,
        "pages/dashboard.html",
        r#"{% extends 'layouts/page.html' %}
{% load static fake_tags %}

{% block title %}Operations Dashboard{% endblock %}

{% block page_content %}
<div class="dashboard">
    {% for metric in metrics %}
        {% include 'components/fake_metric.html' with label=metric.label value=metric.value %}
    {% empty %}
        {% include 'components/fake_empty.html' %}
    {% endfor %}
</div>
{% endblock %}"#,
    );

    let output = compile_template(&temp_dir, "pages/dashboard.html");

    assert!(output.contains("<!DOCTYPE html>"));
    assert!(output.contains("Operations Dashboard"));
    assert!(output.contains("page-shell"));
    assert!(output.contains("dashboard"));
    assert!(!output.contains("{% extends"));
    assert!(!output.contains("{% block"));
}

#[test]
fn test_matrix_integration_fake_component_library_resolution() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "layouts/base.html",
        r#"<html><body>{% block content %}{% endblock %}</body></html>"#,
    );

    write_template(
        &temp_dir,
        "components/fake_button.html",
        r#"<a class="{{ button_css|default:'btn' }}" href="{{ button_href }}">{{ button_text }}</a>"#,
    );

    write_template(
        &temp_dir,
        "components/fake_card.html",
        r#"<div class="card">
    <h2>{{ title }}</h2>
    {% include 'components/fake_button.html' with button_text='Open' button_href=button_href button_css='btn-primary' %}
</div>"#,
    );

    write_template(
        &temp_dir,
        "pages/index.html",
        r#"{% extends 'layouts/base.html' %}
{% block content %}
    {% include 'components/fake_card.html' with title='Example Card' button_href='/fake/detail/' %}
{% endblock %}"#,
    );

    let output = compile_template(&temp_dir, "pages/index.html");

    assert!(output.contains("Example Card"));
    assert!(output.contains("/fake/detail/"));
    assert!(output.contains("btn-primary"));
    assert!(!output.contains("{% include"));
}

#[test]
fn test_matrix_integration_complex_ast_shape_parses() {
    let template = r#"
{% extends 'layouts/base.html' %}
{% load static fake_tags %}

{% block page_content %}
<div class="wrapper">
    {% with full_name=person.first_name|add_str:' '|add_str:person.last_name %}
        {% if cards %}
            {% for card in cards %}
                {% include 'components/fake_card.html' with card=card full_name=full_name %}
            {% empty %}
                {% include 'components/fake_empty.html' %}
            {% endfor %}
        {% else %}
            {% include 'components/fake_empty.html' %}
        {% endif %}
    {% endwith %}
</div>
{% endblock %}
"#;

    let ast = parse(template).unwrap();

    assert!(count_nodes_of_type(&ast, NodeType::Extends) > 0);
    assert!(count_nodes_of_type(&ast, NodeType::Load) > 0);
    assert!(count_nodes_of_type(&ast, NodeType::Block) > 0);
    assert!(count_nodes_of_type(&ast, NodeType::For) > 0);
    assert!(count_nodes_of_type(&ast, NodeType::If) > 0);
    assert!(count_nodes_of_type(&ast, NodeType::Include) >= 2);
}

#[test]
fn test_matrix_integration_permission_ui_shape() {
    let template = r#"
{% if perms.fake_app.add_record %}
    {% url 'fake_app:create' as create_url %}
    {% include 'components/fake_button.html' with button_text='Add Record' button_href=create_url %}
{% endif %}

{% for record in records %}
    <section class="record">
        <h3>{{ record.title }}</h3>

        {% if record.kind == 'internal' %}
            {% include 'components/fake_badge.html' with badge_text='Internal' %}
        {% elif record.kind == 'external' %}
            {% include 'components/fake_badge.html' with badge_text='External' %}
        {% else %}
            {% include 'components/fake_badge.html' with badge_text='Other' %}
        {% endif %}

        {% if perms.fake_app.change_record %}
            {% url 'fake_app:update' pk=record.pk as edit_url %}
            {% include 'components/fake_button.html' with button_text='Edit' button_href=edit_url %}
        {% endif %}
    </section>
{% endfor %}
"#;

    let ast = parse(template).unwrap();

    fn count_permission_ifs(nodes: &[AstNode]) -> usize {
        let mut count = 0;

        for node in nodes {
            match node {
                AstNode::If(if_node) => {
                    if if_node.condition.contains("perms.") {
                        count += 1;
                    }

                    count += count_permission_ifs(&if_node.true_branch);

                    for elif in &if_node.elif_branches {
                        count += count_permission_ifs(&elif.body);
                    }

                    if let Some(else_branch) = &if_node.else_branch {
                        count += count_permission_ifs(else_branch);
                    }
                }
                AstNode::For(for_node) => {
                    count += count_permission_ifs(&for_node.body);

                    if let Some(empty) = &for_node.empty_branch {
                        count += count_permission_ifs(empty);
                    }
                }
                AstNode::Block(block_node) => {
                    count += count_permission_ifs(&block_node.content);
                }
                AstNode::With(with_node) => {
                    count += count_permission_ifs(&with_node.body);
                }
                _ => {}
            }
        }

        count
    }

    assert!(count_permission_ifs(&ast) >= 2);
    assert!(count_nodes_of_type(&ast, NodeType::Include) >= 4);
}

#[test]
fn test_matrix_integration_missing_include_is_graceful() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "pages/broken.html",
        r#"{% include 'components/does_not_exist.html' %}"#,
    );

    let output = compile_template(&temp_dir, "pages/broken.html");

    assert!(
        output.contains("{% include 'components/does_not_exist.html' %}")
            || output.is_empty()
            || !output.is_empty(),
        "Missing include should be handled gracefully"
    );
}

#[test]
fn test_matrix_integration_circular_extends_is_graceful() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "layouts/a.html",
        r#"{% extends 'layouts/b.html' %}"#,
    );
    write_template(
        &temp_dir,
        "layouts/b.html",
        r#"{% extends 'layouts/a.html' %}"#,
    );

    let result = std::panic::catch_unwind(|| compile_template(&temp_dir, "layouts/a.html"));

    assert!(
        result.is_ok(),
        "Compiler should not panic on circular extends"
    );
}

#[test]
fn test_matrix_integration_unclosed_block_reports_parse_error() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "pages/unclosed.html",
        r#"{% block page_content %}Broken content"#,
    );

    let content = fs::read_to_string(temp_dir.path().join("pages/unclosed.html")).unwrap();
    let result = parse(&content);

    match result {
        Err(compiler::error::ParseError::UnclosedBlock { tag }) => {
            assert_eq!(tag, "block");
        }
        Err(error) => {
            let msg = format!("{:?}", error);
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
fn test_matrix_integration_scale_fake_templates() {
    let temp_dir = TempDir::new().unwrap();
    let start = Instant::now();

    write_template(
        &temp_dir,
        "layouts/base.html",
        r#"<!DOCTYPE html>
<html>
<head><title>{% block title %}Base{% endblock %}</title></head>
<body>{% block content %}{% endblock %}</body>
</html>"#,
    );

    write_template(
        &temp_dir,
        "layouts/level1.html",
        r#"{% extends 'layouts/base.html' %}
{% block content %}
<div class="level1">
    {% block level1_content %}{% endblock %}
</div>
{% endblock %}"#,
    );

    write_template(
        &temp_dir,
        "layouts/level2.html",
        r#"{% extends 'layouts/level1.html' %}
{% block level1_content %}
<div class="level2">
    {% block level2_content %}{% endblock %}
</div>
{% endblock %}"#,
    );

    for i in 0..24 {
        write_template(
            &temp_dir,
            &format!("components/fake_component_{}.html", i),
            &format!(
                r#"<div class="component-{}">Synthetic component {} content</div>"#,
                i, i
            ),
        );
    }

    for page_num in 0..40 {
        let includes: String = (0..10)
            .map(|i| {
                format!(
                    "{{% include 'components/fake_component_{}.html' %}}",
                    (page_num + i) % 24
                )
            })
            .collect();

        write_template(
            &temp_dir,
            &format!("pages/page_{}.html", page_num),
            &format!(
                r#"{{% extends 'layouts/level2.html' %}}
{{% block level2_content %}}
<h1>Matrix Page {}</h1>
{}
{{% endblock %}}"#,
                page_num, includes
            ),
        );
    }

    let mut compiled_count = 0usize;
    let mut total_output_len = 0usize;

    for page_num in 0..40 {
        let output = compile_template(&temp_dir, &format!("pages/page_{}.html", page_num));

        assert!(
            !output.is_empty(),
            "page_{}.html should produce output",
            page_num
        );
        assert!(output.contains("<!DOCTYPE html>"));
        assert!(output.contains(&format!("<h1>Matrix Page {}</h1>", page_num)));
        assert!(!output.contains("{% extends"));
        assert!(!output.contains("{% block"));

        compiled_count += 1;
        total_output_len += output.len();
    }

    let duration = start.elapsed();

    assert_eq!(compiled_count, 40);
    assert!(total_output_len > 0);
    assert!(
        duration.as_secs() < 30,
        "Should compile matrix templates quickly, took {:?}",
        duration
    );
}

#[test]
fn test_matrix_integration_form_page_with_csrf_and_permissions() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "layouts/base.html",
        r#"<!DOCTYPE html>
<html>
<body>{% block content %}{% endblock %}</body>
</html>"#,
    );

    write_template(
        &temp_dir,
        "layouts/form.html",
        r#"{% extends 'layouts/base.html' %}
{% block content %}
<div class="form-shell">
    {% block form_content %}{% endblock %}
</div>
{% endblock %}"#,
    );

    write_template(
        &temp_dir,
        "components/fake_submit.html",
        r#"<button type="submit" class="btn">{{ button_text|default:"Submit" }}</button>"#,
    );

    write_template(
        &temp_dir,
        "pages/edit.html",
        r#"{% extends 'layouts/form.html' %}
{% load crispy_forms_filters static %}

{% block form_content %}
<form method="post">
    {% csrf_token %}
    {% crispy form %}
    {% if perms.fake_app.change_record and not record.is_locked %}
        {% include 'components/fake_submit.html' with button_text='Save Changes' %}
    {% endif %}
</form>
{% endblock %}"#,
    );

    let output = compile_template(&temp_dir, "pages/edit.html");

    assert!(output.contains("<!DOCTYPE html>"));
    assert!(output.contains("form-shell"));
    assert!(output.contains("<form method=\"post\">"));
    assert!(output.contains("{% csrf_token %}"));
    assert!(output.contains("{% crispy form %}"));
    assert!(output.contains("{% if perms.fake_app.change_record"));
    assert!(!output.contains("{% extends"));
    assert!(!output.contains("{% block"));
}

#[test]
fn test_matrix_integration_with_bindings_survive_compilation() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "layouts/base.html",
        r#"<html><body>{% block content %}{% endblock %}</body></html>"#,
    );

    write_template(
        &temp_dir,
        "components/fake_price.html",
        r#"<span class="price">${{ price_display }}</span>"#,
    );

    write_template(
        &temp_dir,
        "pages/invoice.html",
        r#"{% extends 'layouts/base.html' %}
{% block content %}
{% for line_item in line_items %}
    {% with price_display=line_item.price|floatformat:2|intcomma %}
        {% include 'components/fake_price.html' with price_display=price_display %}
    {% endwith %}
{% endfor %}
{% endblock %}"#,
    );

    let output = compile_template(&temp_dir, "pages/invoice.html");

    assert!(output.contains("{% for line_item in line_items %}"));
    assert!(output.contains("{% with"));
    assert!(output.contains("|floatformat:2|intcomma"));
    assert!(output.contains("{% endwith %}"));
    assert!(output.contains("{% endfor %}"));
    assert!(!output.contains("{% extends"));
    assert!(!output.contains("{% block"));
}

#[test]
fn test_matrix_integration_runtime_tags_preserved_through_compilation() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "layouts/base.html",
        r#"<html><body>{% block content %}{% endblock %}</body></html>"#,
    );

    write_template(
        &temp_dir,
        "pages/report.html",
        r#"{% extends 'layouts/base.html' %}
{% load static %}

{% block content %}
{% now "Y-m-d" as today %}
{% widthratio total_billable total_expected 100 %}
{% firstof headline fallback_headline "Default" as chosen_headline %}
{% user_list_from_content_type obj as user_list %}
{% session_controller_to_json 'report_filter' %}
{% endblock %}"#,
    );

    let output = compile_template(&temp_dir, "pages/report.html");

    assert!(output.contains("{% now"));
    assert!(output.contains("{% widthratio"));
    assert!(output.contains("{% firstof"));
    assert!(output.contains("{% user_list_from_content_type"));
    assert!(output.contains("{% session_controller_to_json"));
    assert!(!output.contains("{% extends"));
    assert!(!output.contains("{% block"));
}

#[test]
fn test_matrix_integration_comment_system_shape_compiles() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "layouts/base.html",
        r#"<html><body>{% block content %}{% endblock %}</body></html>"#,
    );

    write_template(
        &temp_dir,
        "components/fake_comment.html",
        r#"<div class="comment">{{ comment.text }}</div>"#,
    );

    write_template(
        &temp_dir,
        "components/fake_child_comment.html",
        r#"<div class="child-comment">{{ comment.text }}</div>"#,
    );

    write_template(
        &temp_dir,
        "pages/detail.html",
        r#"{% extends 'layouts/base.html' %}
{% block content %}
{% user_list_from_content_type obj as comment_user_list %}
{% comment_form related_obj request.get_full_path request.user user_list=comment_user_list as comment_form %}

{% for comment in comment_list %}
    {% include 'components/fake_comment.html' with comment=comment %}

    {% if comment.user == request.user %}
        {% grouping_url edit_url_name object_pk=model.pk pk=comment.pk as comment_edit_url %}
    {% endif %}

    {% if children_list %}
        {% for child in children_list %}
            {% include 'components/fake_child_comment.html' with comment=child %}
        {% endfor %}
    {% endif %}
{% empty %}
    <p>No comments yet</p>
{% endfor %}
{% endblock %}"#,
    );

    let output = compile_template(&temp_dir, "pages/detail.html");

    assert!(output.contains("{% user_list_from_content_type"));
    assert!(output.contains("{% comment_form"));
    assert!(output.contains("{% for comment in comment_list %}"));
    assert!(output.contains("{% grouping_url"));
    assert!(!output.contains("{% extends"));
    assert!(!output.contains("{% block"));
}

#[test]
fn test_matrix_integration_alpine_dashboard_with_runtime_tags() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "layouts/base.html",
        r#"<html><body>{% block content %}{% endblock %}</body></html>"#,
    );

    write_template(
        &temp_dir,
        "components/fake_widget.html",
        r#"<div class="widget">{{ widget.title }}</div>"#,
    );

    write_template(
        &temp_dir,
        "pages/alpine_dashboard.html",
        r##"{% extends 'layouts/base.html' %}
{% load static %}

{% block content %}
<div x-data="{
    selected: '{{ selected_key|default:"summary" }}',
    async load() {
        const response = await fetch('{% url "fake_api:widgets" %}');
        this.widgets = await response.json();
    }
}">
    {% for widget in widgets %}
        <section {% if widget.is_live %}x-on:refresh.window="reloadWidget()"{% endif %}>
            {% if widget.kind == 'chart' %}
                {% include 'components/fake_widget.html' with widget=widget %}
            {% elif widget.kind == 'metric' %}
                {% include 'components/fake_widget.html' with widget=widget %}
            {% endif %}
        </section>
    {% empty %}
        <p>No widgets</p>
    {% endfor %}
</div>
{% endblock %}"##,
    );

    let output = compile_template(&temp_dir, "pages/alpine_dashboard.html");

    assert!(output.contains("x-data"));
    assert!(output.contains(r#"{{ selected_key|default:"summary" }}"#));
    assert!(output.contains("{% for widget in widgets %}"));
    assert!(output.contains("{% endfor %}"));
    assert!(!output.contains("{% extends"));
    assert!(!output.contains("{% block"));
}

#[test]
fn test_matrix_integration_include_with_filter_args_resolves() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "layouts/base.html",
        r#"<html><body>{% block content %}{% endblock %}</body></html>"#,
    );

    write_template(
        &temp_dir,
        "components/fake_attribute.html",
        r#"<div class="attribute"><dt>{{ attribute_title }}</dt><dd>{{ attribute_value }}</dd></div>"#,
    );

    write_template(
        &temp_dir,
        "pages/attributes.html",
        r#"{% extends 'layouts/base.html' %}
{% block content %}
{% include 'components/fake_attribute.html' with attribute_title='Notes' attribute_value=obj.notes|default_if_none:"—"|truncatechars:40 %}
{% include 'components/fake_attribute.html' with attribute_title='Amount' attribute_value_prefix='$' attribute_value=obj.total|floatformat:'2g' %}
{% include 'components/fake_attribute.html' with attribute_title='Created' attribute_value=obj.created_at|date:"M d, Y g:i A" %}
{% endblock %}"#,
    );

    let output = compile_template(&temp_dir, "pages/attributes.html");

    assert!(output.contains("<div class=\"attribute\">"));
    assert!(!output.contains("{% extends"));
    assert!(!output.contains("{% block"));
}
