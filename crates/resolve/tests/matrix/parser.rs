use compiler::ast::AstNode;

use crate::common::{NodeType, count_nodes_of_type, parse};

#[test]
fn test_matrix_parser_extends_blocks_and_includes() {
    let template = r#"
{% extends 'layouts/fake_base.html' %}
{% load static fake_tags %}

{% block page_title %}{{ page_title|default:"Example" }}{% endblock %}

{% block page_content %}
<div class="page-shell">
    {% include 'components/fake_header.html' with heading=page_title subtitle='Example subtitle' %}
    {% include 'components/fake_stat.html' with label='Open' value=open_count %}
</div>
{% endblock %}
"#;

    let ast = parse(template).unwrap();

    assert!(count_nodes_of_type(&ast, NodeType::Extends) > 0);
    assert!(count_nodes_of_type(&ast, NodeType::Load) > 0);
    assert!(count_nodes_of_type(&ast, NodeType::Block) >= 2);
    assert!(count_nodes_of_type(&ast, NodeType::Include) >= 2);
}

#[test]
fn test_matrix_parser_nested_if_elif_else_variation() {
    let template = r#"
{% if record.status == 'active' %}
    {% include 'badge/fake_success.html' with text='Active' %}
{% elif record.status == 'pending' %}
    {% include 'badge/fake_warning.html' with text='Pending' %}
{% elif record.status == 'archived' %}
    {% include 'badge/fake_secondary.html' with text='Archived' %}
{% else %}
    {% include 'badge/fake_danger.html' with text='Unknown' %}
{% endif %}
"#;

    let ast = parse(template).unwrap();

    assert!(count_nodes_of_type(&ast, NodeType::If) > 0);

    let has_elif = ast.iter().any(|node| {
        if let AstNode::If(if_node) = node {
            !if_node.elif_branches.is_empty()
        } else {
            false
        }
    });

    assert!(has_elif, "Expected at least one elif branch");
    assert!(count_nodes_of_type(&ast, NodeType::Include) >= 4);
}

#[test]
fn test_matrix_parser_for_empty_and_nested_if() {
    let template = r#"
{% for card in cards %}
    <article class="card">
        {% if card.is_featured %}
            {% include 'cards/fake_featured.html' with card=card %}
        {% else %}
            {% include 'cards/fake_standard.html' with card=card %}
        {% endif %}
    </article>
{% empty %}
    {% include 'cards/fake_empty.html' %}
{% endfor %}
"#;

    let ast = parse(template).unwrap();

    assert!(count_nodes_of_type(&ast, NodeType::For) > 0);
    assert!(count_nodes_of_type(&ast, NodeType::If) > 0);
    assert!(count_nodes_of_type(&ast, NodeType::Include) >= 3);

    let has_empty = ast.iter().any(|node| {
        if let AstNode::For(for_node) = node {
            for_node.empty_branch.is_some()
        } else {
            false
        }
    });

    assert!(has_empty, "Expected for-loop empty branch");
}

#[test]
fn test_matrix_parser_tuple_unpacking_and_filters() {
    let template = r#"
{% for key, value in rows.items %}
    <div class="row">
        <span>{{ key|default:"unknown_key" }}</span>
        <span>{{ value|default:"unknown_value"|upper }}</span>
    </div>
{% empty %}
    <p>No rows</p>
{% endfor %}
"#;

    let ast = parse(template).unwrap();

    let has_tuple_unpacking = ast.iter().any(|node| {
        if let AstNode::For(for_node) = node {
            for_node.variable.contains(",")
        } else {
            false
        }
    });

    assert!(has_tuple_unpacking, "Expected tuple unpacking in for loop");

    let variable_with_filters = ast.iter().any(|node| {
        if let AstNode::For(for_node) = node {
            for_node.body.iter().any(|body_node| {
                if let AstNode::Variable(variable) = body_node {
                    !variable.filters.is_empty()
                } else {
                    false
                }
            })
        } else {
            false
        }
    });

    assert!(
        variable_with_filters,
        "Expected filtered variables inside loop body"
    );
}

#[test]
fn test_matrix_parser_with_bindings_and_filter_chains() {
    let template = r#"
{% with full_name=person.first_name|add_str:' '|add_str:person.last_name %}
    <p>{{ full_name|default:"Anonymous User" }}</p>
{% endwith %}

{% with phone_href='tel:'|add_str:person.phone email_href='mailto:'|add_str:person.email %}
    <a href="{{ phone_href }}">{{ person.phone }}</a>
    <a href="{{ email_href }}">{{ person.email }}</a>
{% endwith %}
"#;

    let ast = parse(template).unwrap();

    let with_blocks: Vec<_> = ast
        .iter()
        .filter(|node| matches!(node, AstNode::With(_)))
        .collect();

    assert_eq!(with_blocks.len(), 2, "Expected two with blocks");

    let has_filter_binding = ast.iter().any(|node| {
        if let AstNode::With(with_node) = node {
            with_node
                .bindings
                .iter()
                .any(|binding| binding.value.contains("|"))
        } else {
            false
        }
    });

    assert!(
        has_filter_binding,
        "Expected filter chain inside with binding"
    );
}

#[test]
fn test_matrix_parser_blocks_inside_html_attributes() {
    let template = r#"
<div class="{% block shell_class %}shell-default{% endblock %} {{ extra_class|default:'shell-base' }}">
    <span data-state="{% if record.is_enabled %}enabled{% else %}disabled{% endif %}">
        {{ record.label }}
    </span>
</div>
"#;

    let ast = parse(template).unwrap();

    assert!(count_nodes_of_type(&ast, NodeType::Block) > 0);
    assert!(count_nodes_of_type(&ast, NodeType::If) > 0);
}

#[test]
fn test_matrix_parser_dashboard_shape() {
    let template = r##"
{% extends 'layouts/dashboard.html' %}
{% load static fake_ui %}

{% block page_content %}
<div class="dashboard"
     x-data="{
        refreshInterval: {{ refresh_interval|default:15000 }},
        selectedKey: '{{ selected_key|default:"summary" }}'
     }">

    {% for widget in widgets %}
        <section class="widget" {% if widget.is_live %}x-on:refresh.window="reloadWidget()"{% endif %}>
            <h3>{{ widget.title }}</h3>

            {% if widget.kind == 'chart' %}
                {% include 'widgets/fake_chart.html' with config=widget.config %}
            {% elif widget.kind == 'table' %}
                {% include 'widgets/fake_table.html' with rows=widget.rows %}
            {% elif widget.kind == 'metric' %}
                {% include 'widgets/fake_metric.html' with value=widget.value %}
            {% else %}
                {% include 'widgets/fake_unknown.html' %}
            {% endif %}
        </section>
    {% empty %}
        {% include 'widgets/fake_empty.html' %}
    {% endfor %}
</div>
{% endblock %}
"##;

    let ast = parse(template).unwrap();

    assert!(count_nodes_of_type(&ast, NodeType::Extends) > 0);
    assert!(count_nodes_of_type(&ast, NodeType::Load) > 0);
    assert!(count_nodes_of_type(&ast, NodeType::Block) > 0);
    assert!(count_nodes_of_type(&ast, NodeType::For) > 0);
    assert!(count_nodes_of_type(&ast, NodeType::If) > 0);
    assert!(count_nodes_of_type(&ast, NodeType::Include) >= 4);
}

#[test]
fn test_matrix_parser_permission_if_with_and_not_filter() {
    let template = r#"
{% if perms.app_a.change_model_a and not obj_a.status|in_list:'done,rejected,cancelled' %}
    {% include 'components/fake_edit_button.html' with href=edit_url %}
{% endif %}
"#;

    let ast = parse(template).unwrap();

    assert!(count_nodes_of_type(&ast, NodeType::If) > 0);

    let has_complex_condition = ast.iter().any(|node| {
        if let AstNode::If(if_node) = node {
            if_node.condition.contains("perms.")
                && if_node.condition.contains("and not")
                && if_node.condition.contains("|in_list:")
        } else {
            false
        }
    });

    assert!(
        has_complex_condition,
        "Expected complex permission condition with and not + filter"
    );
    assert!(count_nodes_of_type(&ast, NodeType::Include) >= 1);
}

#[test]
fn test_matrix_parser_permission_if_with_multiple_or() {
    let template = r#"
{% if perms.app_e.view_x or perms.app_e.view_y or perms.app_e.view_z %}
    {% include 'components/fake_panel.html' %}
{% endif %}
"#;

    let ast = parse(template).unwrap();

    let has_multi_or = ast.iter().any(|node| {
        if let AstNode::If(if_node) = node {
            let or_count = if_node.condition.matches(" or ").count();
            or_count >= 2
        } else {
            false
        }
    });

    assert!(
        has_multi_or,
        "Expected if condition with multiple or clauses"
    );
}

#[test]
fn test_matrix_parser_permission_if_complex_and_or_chain() {
    let template = r#"
{% if perms.app_b.delete_model_b or perms.app_c.add_model_c and request.user.id == obj_a.requestor_id and obj_a.phase == 'req' %}
    {% include 'components/fake_action_button.html' %}
{% endif %}
"#;

    let ast = parse(template).unwrap();

    let has_mixed_logic = ast.iter().any(|node| {
        if let AstNode::If(if_node) = node {
            if_node.condition.contains(" or ")
                && if_node.condition.contains(" and ")
                && if_node.condition.contains("==")
        } else {
            false
        }
    });

    assert!(
        has_mixed_logic,
        "Expected if condition with mixed and/or and comparison"
    );
}

#[test]
fn test_matrix_parser_status_elif_chain_with_enum_constants() {
    let template = r#"
{% if obj_a.stage == StageChoices.IDENTIFY %}
    {% include 'badge/fake_identify.html' %}
{% elif obj_a.stage == StageChoices.CONNECT %}
    {% include 'badge/fake_connect.html' %}
{% elif obj_a.stage == StageChoices.EXPLORE %}
    {% include 'badge/fake_explore.html' %}
{% elif obj_a.stage == StageChoices.ADVISE %}
    {% include 'badge/fake_advise.html' %}
{% elif obj_a.stage == StageChoices.SENT %}
    {% include 'badge/fake_sent.html' %}
{% elif obj_a.stage == StageChoices.WON %}
    {% include 'badge/fake_won.html' %}
{% elif obj_a.stage == StageChoices.LOST %}
    {% include 'badge/fake_lost.html' %}
{% endif %}
"#;

    let ast = parse(template).unwrap();

    let has_many_elifs = ast.iter().any(|node| {
        if let AstNode::If(if_node) = node {
            if_node.elif_branches.len() >= 6
        } else {
            false
        }
    });

    assert!(
        has_many_elifs,
        "Expected if with 6+ elif branches for enum comparison"
    );
    assert!(count_nodes_of_type(&ast, NodeType::Include) >= 7);
}

#[test]
fn test_matrix_parser_if_with_emoji_string_comparison() {
    let template = r#"
{% if '🎆 Done' in status %}
    {% include 'badge/fake_done.html' %}
{% elif '🤔 Waiting Information' in status %}
    {% include 'badge/fake_waiting.html' %}
{% elif '🔎 Code Review' in status %}
    {% include 'badge/fake_review.html' %}
{% elif '👷‍♂️ Ready' in status %}
    {% include 'badge/fake_ready.html' %}
{% elif '🏗️ In Progress' in status %}
    {% include 'badge/fake_progress.html' %}
{% endif %}
"#;

    let ast = parse(template).unwrap();

    assert!(count_nodes_of_type(&ast, NodeType::If) > 0);

    let has_emoji_condition = ast.iter().any(|node| {
        if let AstNode::If(if_node) = node {
            if_node.condition.contains("🎆")
        } else {
            false
        }
    });

    assert!(
        has_emoji_condition,
        "Expected if condition with emoji string"
    );
}

#[test]
fn test_matrix_parser_if_with_date_filter_comparison() {
    let template = r#"
{% if issue.due_date|date:'Y-m-d' < today %}
    {% include 'badge/fake_overdue.html' %}
{% elif issue.due_date|date:'Y-m-d' == today %}
    {% include 'badge/fake_due_today.html' %}
{% endif %}
"#;

    let ast = parse(template).unwrap();

    let has_filter_comparison = ast.iter().any(|node| {
        if let AstNode::If(if_node) = node {
            if_node.condition.contains("|date:") && if_node.condition.contains("< today")
        } else {
            false
        }
    });

    assert!(
        has_filter_comparison,
        "Expected if condition with date filter and comparison operator"
    );
}

#[test]
fn test_matrix_parser_for_with_filtered_iterable() {
    let template = r#"
{% for assignment in dispatch.assignments|get_item:column %}
    {% include 'components/fake_assignment_card.html' with assignment=assignment %}
{% endfor %}
"#;

    let ast = parse(template).unwrap();

    let has_filtered_iterable = ast.iter().any(|node| {
        if let AstNode::For(for_node) = node {
            for_node.iterable.contains("|get_item:")
        } else {
            false
        }
    });

    assert!(
        has_filtered_iterable,
        "Expected for-loop with filter applied to iterable"
    );
}

#[test]
fn test_matrix_parser_for_tuple_unpacking_deep_access() {
    let template = r#"
{% for material, load_counts in daily_report_entity.load_summaries.items %}
    <div>{{ material }}: {{ load_counts }}</div>
{% endfor %}
"#;

    let ast = parse(template).unwrap();

    let has_deep_unpacking = ast.iter().any(|node| {
        if let AstNode::For(for_node) = node {
            for_node.variable.contains(",")
                && for_node
                    .iterable
                    .contains("daily_report_entity.load_summaries.items")
        } else {
            false
        }
    });

    assert!(
        has_deep_unpacking,
        "Expected tuple unpacking with deeply nested iterable"
    );
}

#[test]
fn test_matrix_parser_with_as_syntax_filter_chain() {
    let template = r#"
{% with employee.latest_feedback_datetime|date:'F j, Y'|default:"None" as feedback_date %}
    <p>{{ feedback_date }}</p>
{% endwith %}
"#;

    let ast = parse(template).unwrap();

    let has_with = ast.iter().any(|node| matches!(node, AstNode::With(_)));
    assert!(
        has_with,
        "Expected With node for as-syntax with filter chain"
    );
}

#[test]
fn test_matrix_parser_with_string_concatenation_as() {
    let template = r#"
{% with ''|add:total_count|add:' / '|add:error_count as count_display %}
    <span>{{ count_display }}</span>
{% endwith %}
"#;

    let ast = parse(template).unwrap();

    let has_with = ast.iter().any(|node| matches!(node, AstNode::With(_)));
    assert!(
        has_with,
        "Expected With node for string concatenation pattern"
    );
}

#[test]
fn test_matrix_parser_with_model_filter_bindings() {
    let template = r#"
{% with app_label=obj|model_app_label model_name=obj|model_name %}
    <p>{{ app_label }}.{{ model_name }}</p>
{% endwith %}
"#;

    let ast = parse(template).unwrap();

    let has_multi_filter_bindings = ast.iter().any(|node| {
        if let AstNode::With(with_node) = node {
            with_node.bindings.len() >= 2
                && with_node
                    .bindings
                    .iter()
                    .all(|binding| binding.value.contains("|"))
        } else {
            false
        }
    });

    assert!(
        has_multi_filter_bindings,
        "Expected with block with multiple filter-chain bindings"
    );
}

#[test]
fn test_matrix_parser_with_floatformat_intcomma_chain() {
    let template = r#"
{% with price_display=line_item.price|floatformat:2|intcomma %}
    <span>${{ price_display }}</span>
{% endwith %}
"#;

    let ast = parse(template).unwrap();

    let has_chained_binding = ast.iter().any(|node| {
        if let AstNode::With(with_node) = node {
            with_node.bindings.iter().any(|binding| {
                binding.value.contains("|floatformat:") && binding.value.contains("|intcomma")
            })
        } else {
            false
        }
    });

    assert!(
        has_chained_binding,
        "Expected with binding with floatformat + intcomma chain"
    );
}

#[test]
fn test_matrix_parser_include_with_complex_filter_chains_in_with() {
    let template = r#"
{% include 'pkg_a/comp_c/elem_l.html' with badge_text=obj_a.owner.first_name|slice:":1"|add_str:obj_a.owner.last_name|slice:":2" %}
{% include 'pkg_a/comp_c/elem_o.html' with attribute_title='FieldC' attribute_value_prefix='$' attribute_value=obj_a.total|floatformat:'2g' %}
{% include 'pkg_a/comp_c/elem_p.html' with attribute_title='FieldD' attribute_value=obj_a.created_at|date:"M d, Y g:i A" %}
"#;

    let ast = parse(template).unwrap();

    assert!(count_nodes_of_type(&ast, NodeType::Include) >= 3);
}

#[test]
fn test_matrix_parser_widthratio_node() {
    let template = "{% widthratio total_billable_hours total_expected_hours 100 %}";
    let ast = parse(template).unwrap();

    assert!(matches!(ast[0], AstNode::Widthratio(_)));

    if let AstNode::Widthratio(node) = &ast[0] {
        assert_eq!(node.value, "total_billable_hours");
        assert_eq!(node.maximum, "total_expected_hours");
        assert_eq!(node.divisor, "100");
    } else {
        panic!("Expected Widthratio node");
    }
}

#[test]
fn test_matrix_parser_firstof_node() {
    let template = "{% firstof attribute_value or x_attribute_value as value %}";
    let ast = parse(template).unwrap();

    assert!(matches!(ast[0], AstNode::Firstof(_)));
}

#[test]
fn test_matrix_parser_now_with_as() {
    let template = r#"{% now "Y-m-d" as today %}"#;
    let ast = parse(template).unwrap();

    assert!(matches!(ast[0], AstNode::Now(_)));
}

#[test]
fn test_matrix_parser_form_page_shape() {
    let template = r##"
{% extends 'layouts/fake_form.html' %}
{% load static crispy_forms_filters %}

{% block page_content %}
<form method="post" action="{% url 'fake_ns:record:update' pk=record.pk %}">
    {% csrf_token %}

    {% crispy form %}

    {% if perms.fake_app.change_record and not record.is_locked %}
        {% include 'components/fake_submit_button.html' with button_text='Save Changes' %}
    {% endif %}

    {% if perms.fake_app.delete_record %}
        {% url 'fake_ns:record:delete' pk=record.pk as delete_url %}
        {% include 'components/fake_delete_button.html' with button_href=delete_url %}
    {% endif %}
</form>
{% endblock %}
"##;

    let ast = parse(template).unwrap();

    assert!(count_nodes_of_type(&ast, NodeType::Extends) > 0);
    assert!(count_nodes_of_type(&ast, NodeType::Load) > 0);
    assert!(count_nodes_of_type(&ast, NodeType::Block) > 0);
    assert!(count_nodes_of_type(&ast, NodeType::Csrftoken) > 0);
    assert!(count_nodes_of_type(&ast, NodeType::If) >= 2);
    assert!(count_nodes_of_type(&ast, NodeType::Include) >= 2);
}

#[test]
fn test_matrix_parser_nested_with_inside_for() {
    let template = r#"
{% for line_item in line_items %}
    {% with price_display=line_item.price|floatformat:2|intcomma cost_display=line_item.total_cost|floatformat:2|intcomma %}
        <div class="line-item">
            <span>${{ price_display }}</span>
            <span>${{ cost_display }}</span>
        </div>
    {% endwith %}
{% endfor %}
"#;

    let ast = parse(template).unwrap();

    assert!(count_nodes_of_type(&ast, NodeType::For) > 0);

    let has_with_inside_for = ast.iter().any(|node| {
        if let AstNode::For(for_node) = node {
            for_node
                .body
                .iter()
                .any(|body_node| matches!(body_node, AstNode::With(_)))
        } else {
            false
        }
    });

    assert!(
        has_with_inside_for,
        "Expected with block nested inside for-loop"
    );
}

#[test]
fn test_matrix_parser_comment_system_shape() {
    let template = r#"
{% user_list_from_content_type obj_a as comment_user_list %}
{% comment_form related_obj request.get_full_path request.user user_list=comment_user_list as comment_form %}

{% for comment in comment_list %}
    <div class="comment">
        <p>{{ comment.text }}</p>

        {% if comment.user == request.user %}
            {% grouping_url edit_url_name object_pk=model.pk pk=comment.pk as comment_edit_url %}
            {% include 'components/fake_edit_link.html' with href=comment_edit_url %}
        {% endif %}

        {% if children_list %}
            {% for child in children_list %}
                {% include 'components/fake_child_comment.html' with comment=child %}
            {% endfor %}
        {% endif %}
    </div>
{% empty %}
    {% if not has_comment_permission %}
        <p>No comments</p>
    {% endif %}
{% endfor %}
"#;

    let ast = parse(template).unwrap();

    assert!(count_nodes_of_type(&ast, NodeType::For) >= 1);
    assert!(count_nodes_of_type(&ast, NodeType::If) >= 2);
    assert!(count_nodes_of_type(&ast, NodeType::Include) >= 2);
}
