use crate::common::compile_simple;

#[test]
fn test_matrix_codegen_custom_filters_preserved() {
    let template = r#"{{ person.first_name|add_str:' '|add_str:person.last_name }}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
    assert_eq!(output.matches("|add_str:").count(), 2);
}

#[test]
fn test_matrix_codegen_custom_filter_family_preserved() {
    let template =
        r#"{{ record|model_app_label }} {{ record|model_name }} {{ payload|safe_dict_items }}"#;
    let output = compile_simple(template);

    assert!(output.contains("|model_app_label"));
    assert!(output.contains("|model_name"));
    assert!(output.contains("|safe_dict_items"));
}

#[test]
fn test_matrix_codegen_if_with_in_list_preserved() {
    let template = r#"{% if state_code|in_list:'new,pending,closed' %}Allowed{% endif %}"#;
    let output = compile_simple(template);

    assert!(output.contains("{% if"));
    assert!(output.contains("|in_list:'new,pending,closed'"));
    assert!(output.contains("{% endif %}"));
}

#[test]
fn test_matrix_codegen_generate_id_preserved() {
    let template = r#"{% generate_id as fake_dom_id %}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_pagination_url_preserved() {
    let template = r#"{% pagination_url 1 as first_page_url %}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_grouping_url_preserved() {
    let template = r#"{% grouping_url fake_comment_edit_url object_pk=model.pk pk=note.pk as comment_edit_url %}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_form_url_preserved() {
    let template =
        r#"{% form_url 'fake_ns:record:update' pk=record.pk owner_pk=user.pk as edit_form_url %}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_content_type_url_preserved() {
    let template = r#"{% content_type_url 'fake_comment:form' obj comment_pk=0 obj_pk=obj.pk return_url=request.path %}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_comment_form_preserved() {
    let template = r#"{% comment_form related_obj request.path request.user parent=note user_list=reviewer_list as note_form %}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_session_controller_to_json_preserved() {
    let template = r#"{% session_controller_to_json fake_filter_session_key %}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_django_messages_to_json_preserved() {
    let template = r#"{% django_messages_to_json messages %}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_get_elided_page_range_preserved() {
    let template = r#"{% get_elided_page_range page_obj as page_range %}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_now_firstof_widthratio_preserved() {
    let template = r#"
{% now "Y-m-d" as today_value %}
{% firstof headline fallback_headline "Fallback headline" as chosen_headline %}
{% widthratio completed total 100 %}
"#;
    let output = compile_simple(template);

    assert!(output.contains(r#"{% now "Y-m-d" as today_value %}"#));
    assert!(output.contains("{% firstof"));
    assert!(output.contains("{% widthratio"));
}

#[test]
fn test_matrix_codegen_url_and_include_shapes_preserved() {
    let template = r#"
{% url 'fake_ns:record:detail' pk=record.pk as detail_url %}
{% include 'components/fake_button.html' with button_text='Open' button_href=detail_url button_css='btn-primary' %}
"#;

    let output = compile_simple(template);

    assert!(output.contains("{% url 'fake_ns:record:detail' pk=record.pk as detail_url %}"));
    assert!(output.contains(
        "{% include 'components/fake_button.html' with button_text='Open' button_href=detail_url button_css='btn-primary' %}"
    ));
}

#[test]
fn test_matrix_codegen_fake_static_paths_preserved() {
    let template = r#"<script src="{% static 'assets/js/fake_app.js' %}?v=2026-04-02"></script>"#;
    let output = compile_simple(template);

    assert!(output.contains("{% static 'assets/js/fake_app.js' %}"));
    assert!(output.contains("?v=2026-04-02"));
}

#[test]
fn test_matrix_codegen_alpine_and_runtime_tags_preserved() {
    let template = r##"
<div x-data="{
    selected: '{{ selected_key|default:"summary" }}',
    async load() {
        const response = await fetch('{% url "fake_api:items" %}');
        this.items = await response.json();
    }
}">
    {% if is_ready %}
        {% include 'components/fake_ready_state.html' %}
    {% endif %}
</div>
"##;

    let output = compile_simple(template);

    assert!(output.contains("x-data"));
    assert!(output.contains(r#"{{ selected_key|default:"summary" }}"#));
    assert!(
        output.contains(r#"{% url "fake_api:items" %}"#)
            || output.contains("{% url 'fake_api:items' %}")
    );
    assert!(output.contains("{% if is_ready %}"));
    assert!(output.contains("{% include 'components/fake_ready_state.html' %}"));
    assert!(output.contains("{% endif %}"));
}

#[test]
fn test_matrix_codegen_form_label_to_name_preserved() {
    let template = r#"{% form_label_to_name label_value %}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_string_to_font_scale_preserved() {
    let template = r#"{% string_to_font_scale heading_value char_limit %}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_long_filter_chain_preserved() {
    let template = r#"{{ var|f1|f2|f3|f4|f5|f6|f7|f8|f9|f10 }}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
    assert_eq!(output.matches("|f").count(), 10);
}

#[test]
fn test_matrix_codegen_literal_string_prefix_variable_preserved() {
    let template = r#"{{ "$"|add:price_display }}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_slice_add_str_chain_preserved() {
    let template =
        r#"{{ obj_a.owner.first_name|slice:":1"|add_str:obj_a.owner.last_name|slice:":2" }}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
    assert_eq!(output.matches("|slice:").count(), 2);
}

#[test]
fn test_matrix_codegen_default_if_none_unicode_preserved() {
    let template = r#"{{ value|default_if_none:"—"|truncatechars:40 }}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_yesno_filter_preserved() {
    let template = r#"{{ value|yesno:"Yes,No" }}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_floatformat_string_arg_preserved() {
    let template = r#"{{ value|floatformat:"2g" }}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_widthratio_preserved() {
    let template = r#"{% widthratio total_billable_hours total_expected_hours 100 %}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_user_list_from_content_type_preserved() {
    let template = r#"{% user_list_from_content_type obj_a as comment_user_list %}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_crispy_preserved() {
    let template = r#"{% crispy form %}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_url_with_many_kwargs_and_as_preserved() {
    let template = r#"{% url 'fake_ns:view_g' app_id=app_id deployment_id=active_id log_type='BUILD' as build_url %}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_url_with_filter_in_kwarg_preserved() {
    let template = r#"{% url 'fake_ns:view_ag' pk=obj_a.pk|default:0 as form_url %}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_with_floatformat_intcomma_preserved() {
    let template = r#"{% with price_display=line_item.price|floatformat:2|intcomma %}${{ price_display }}{% endwith %}"#;
    let output = compile_simple(template);

    assert!(output.contains("{% with"));
    assert!(output.contains("|floatformat:2|intcomma"));
    assert!(output.contains("{% endwith %}"));
}

#[test]
fn test_matrix_codegen_with_model_filter_bindings_preserved() {
    let template = r#"{% with app_label=obj|model_app_label model_name=obj|model_name %}{{ app_label }}.{{ model_name }}{% endwith %}"#;
    let output = compile_simple(template);

    assert!(output.contains("{% with"));
    assert!(output.contains("app_label=obj|model_app_label"));
    assert!(output.contains("model_name=obj|model_name"));
    assert!(output.contains("{% endwith %}"));
}

#[test]
fn test_matrix_codegen_if_permission_and_not_filter_preserved() {
    let template = r#"{% if perms.app_a.change_model_a and not obj_a.status|in_list:'done,rejected,cancelled' %}Allowed{% endif %}"#;
    let output = compile_simple(template);

    assert!(output.contains("{% if"));
    assert!(output.contains("perms.app_a.change_model_a"));
    assert!(output.contains("and not"));
    assert!(output.contains("|in_list:'done,rejected,cancelled'"));
    assert!(output.contains("{% endif %}"));
}

#[test]
fn test_matrix_codegen_if_multiple_or_preserved() {
    let template = r#"{% if perms.app_e.view_x or perms.app_e.view_y or perms.app_e.view_z %}Visible{% endif %}"#;
    let output = compile_simple(template);

    assert!(output.contains("{% if"));
    assert!(output.contains("or perms.app_e.view_y"));
    assert!(output.contains("or perms.app_e.view_z"));
    assert!(output.contains("{% endif %}"));
}

#[test]
fn test_matrix_codegen_include_with_filter_chain_in_with_preserved() {
    let template = r#"{% include 'pkg_a/comp_c/elem_f.html' with value=obj_a.notes|default_if_none:"—"|truncatechars:40 %}"#;
    let output = compile_simple(template);

    assert!(output.contains("{% include"));
    assert!(output.contains("|default_if_none:"));
    assert!(output.contains("|truncatechars:40"));
}

#[test]
fn test_matrix_codegen_include_with_floatformat_suffix_preserved() {
    let template = r#"{% include 'pkg_a/comp_c/elem_e.html' with value=obj_a.amount|floatformat:2 suffix=' CUR' %}"#;
    let output = compile_simple(template);

    assert!(output.contains("{% include"));
    assert!(output.contains("|floatformat:2"));
    assert!(output.contains("suffix=' CUR'"));
}

#[test]
fn test_matrix_codegen_for_with_filtered_iterable_preserved() {
    let template = r#"{% for assignment in dispatch.assignments|get_item:column %}{{ assignment }}{% endfor %}"#;
    let output = compile_simple(template);

    assert!(output.contains("{% for"));
    assert!(output.contains("|get_item:column"));
    assert!(output.contains("{% endfor %}"));
}

#[test]
fn test_matrix_codegen_for_tuple_unpacking_deep_preserved() {
    let template = r#"{% for material, load_counts in daily_report_entity.load_summaries.items %}{{ material }}{% endfor %}"#;
    let output = compile_simple(template);

    assert!(output.contains("{% for"));
    assert!(output.contains("material, load_counts"));
    assert!(output.contains("daily_report_entity.load_summaries.items"));
    assert!(output.contains("{% endfor %}"));
}

#[test]
fn test_matrix_codegen_load_multiple_libraries_preserved() {
    let template = r#"{% load crispy_forms_filters static %}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_session_controller_string_key_preserved() {
    let template = r#"{% session_controller_to_json 'task_list_filter' %}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_if_emoji_string_preserved() {
    let template = r#"{% if '🏗️ In Progress' in status %}Active{% endif %}"#;
    let output = compile_simple(template);

    assert!(output.contains("{% if"));
    assert!(output.contains("🏗️ In Progress"));
    assert!(output.contains("{% endif %}"));
}

#[test]
fn test_matrix_codegen_if_date_filter_comparison_preserved() {
    let template = r#"{% if issue.due_date|date:'Y-m-d' < today %}Overdue{% endif %}"#;
    let output = compile_simple(template);

    assert!(output.contains("{% if"));
    assert!(output.contains("|date:'Y-m-d'"));
    assert!(output.contains("< today"));
    assert!(output.contains("{% endif %}"));
}

#[test]
fn test_matrix_codegen_now_with_as_preserved() {
    let template = r#"{% now "Y-m-d" as today %}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_firstof_with_or_and_as_preserved() {
    let template = r#"{% firstof attribute_value or x_attribute_value as value %}"#;
    let output = compile_simple(template);

    assert_eq!(output.trim(), template.trim());
}

#[test]
fn test_matrix_codegen_with_dashes_and_spaces_to_underscore_preserved() {
    let template = r#"{% with 'display_'|add:key|dashes_and_spaces_to_underscore as key_display %}{{ key_display }}{% endwith %}"#;
    let output = compile_simple(template);

    assert!(output.contains("{% with"));
    assert!(output.contains("|add:key"));
    assert!(output.contains("|dashes_and_spaces_to_underscore"));
    assert!(output.contains("as key_display"));
    assert!(output.contains("{% endwith %}"));
}
