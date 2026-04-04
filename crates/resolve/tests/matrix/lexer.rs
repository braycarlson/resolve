use compiler::lexer::{Lexer, Token};

#[test]
fn test_matrix_lexer_variable_add_str_chain() {
    let mut lexer = Lexer::new("{{ person.first_name|add_str:' '|add_str:person.last_name }}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::Variable {
            expression,
            filters,
            raw,
        } => {
            assert_eq!(
                *expression,
                "person.first_name|add_str:' '|add_str:person.last_name"
            );
            assert_eq!(filters.len(), 2);
            assert_eq!(filters[0].name, "add_str");
            assert_eq!(filters[0].arguments, vec!["' '"]);
            assert_eq!(filters[1].name, "add_str");
            assert_eq!(filters[1].arguments, vec!["person.last_name"]);
            assert_eq!(
                *raw,
                "{{ person.first_name|add_str:' '|add_str:person.last_name }}"
            );
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_matrix_lexer_variable_in_list_filter() {
    let mut lexer = Lexer::new("{{ state_code|in_list:'new,pending,closed' }}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::Variable { filters, .. } => {
            assert_eq!(filters.len(), 1);
            assert_eq!(filters[0].name, "in_list");
            assert_eq!(filters[0].arguments, vec!["'new,pending,closed'"]);
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_matrix_lexer_custom_tag_generate_id() {
    let mut lexer = Lexer::new("{% generate_id as fake_dom_id %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "generate_id");
            assert!(content.contains("as fake_dom_id"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_custom_tag_pagination_url() {
    let mut lexer = Lexer::new("{% pagination_url 1 as first_page_url %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "pagination_url");
            assert!(content.contains("1"));
            assert!(content.contains("as first_page_url"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_custom_tag_grouping_url() {
    let mut lexer = Lexer::new(
        "{% grouping_url fake_comment_edit_url object_pk=model.pk pk=note.pk as comment_edit_url %}",
    );
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "grouping_url");
            assert!(content.contains("fake_comment_edit_url"));
            assert!(content.contains("object_pk=model.pk"));
            assert!(content.contains("pk=note.pk"));
            assert!(content.contains("as comment_edit_url"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_custom_tag_form_url() {
    let mut lexer = Lexer::new(
        "{% form_url 'fake_ns:record:update' pk=record.pk owner_pk=user.pk as edit_form_url %}",
    );
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "form_url");
            assert!(content.contains("'fake_ns:record:update'"));
            assert!(content.contains("pk=record.pk"));
            assert!(content.contains("owner_pk=user.pk"));
            assert!(content.contains("as edit_form_url"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_custom_tag_content_type_url() {
    let mut lexer = Lexer::new(
        "{% content_type_url 'fake_comment:form' obj comment_pk=0 obj_pk=obj.pk return_url=request.path %}",
    );
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "content_type_url");
            assert!(content.contains("'fake_comment:form'"));
            assert!(content.contains("obj"));
            assert!(content.contains("comment_pk=0"));
            assert!(content.contains("obj_pk=obj.pk"));
            assert!(content.contains("return_url=request.path"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_custom_tag_comment_form() {
    let mut lexer = Lexer::new(
        "{% comment_form related_obj request.path request.user parent=note user_list=reviewer_list as note_form %}",
    );
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "comment_form");
            assert!(content.contains("related_obj"));
            assert!(content.contains("request.path"));
            assert!(content.contains("request.user"));
            assert!(content.contains("parent=note"));
            assert!(content.contains("user_list=reviewer_list"));
            assert!(content.contains("as note_form"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_custom_tag_form_label_to_name() {
    let mut lexer = Lexer::new("{% form_label_to_name label_value %}");
    let tokens = lexer.tokenize().unwrap();

    assert!(matches!(
        &tokens[0],
        Token::BlockStart { tag, .. } if *tag == "form_label_to_name"
    ));
}

#[test]
fn test_matrix_lexer_custom_tag_string_to_font_scale() {
    let mut lexer = Lexer::new("{% string_to_font_scale heading_value char_limit %}");
    let tokens = lexer.tokenize().unwrap();

    assert!(matches!(
        &tokens[0],
        Token::BlockStart { tag, .. } if *tag == "string_to_font_scale"
    ));
}

#[test]
fn test_matrix_lexer_alpine_with_fake_django_data() {
    let template =
        r#"<div x-data="{ selected: '{{ selected_key|default:"summary" }}', open: false }"></div>"#;
    let mut lexer = Lexer::new(template);
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
    assert!(
        tokens
            .iter()
            .any(|token| matches!(token, Token::Text(text) if text.contains("x-data")))
    );
}

#[test]
fn test_matrix_lexer_alpine_complex_fetch_and_url() {
    let template = r#"<div x-data="{
    async load() {
        const response = await fetch('{% url "fake_api:items" %}');
        this.items = await response.json();
    }
}"></div>"#;

    let mut lexer = Lexer::new(template);
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
    assert!(
        tokens
            .iter()
            .any(|token| matches!(token, Token::Text(text) if text.contains("fetch(")))
    );
}

#[test]
fn test_matrix_lexer_long_filter_chain() {
    let mut lexer = Lexer::new("{{ var|f1|f2|f3|f4|f5|f6|f7|f8|f9|f10 }}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::Variable { filters, .. } => {
            assert_eq!(filters.len(), 10);
            assert_eq!(filters[0].name, "f1");
            assert_eq!(filters[4].name, "f5");
            assert_eq!(filters[9].name, "f10");
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_matrix_lexer_variable_literal_string_prefix() {
    let mut lexer = Lexer::new(r#"{{ "$"|add:price_display }}"#);
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::Variable {
            expression,
            filters,
            ..
        } => {
            assert!(expression.contains("\"$\""));
            assert_eq!(filters.len(), 1);
            assert_eq!(filters[0].name, "add");
            assert_eq!(filters[0].arguments, vec!["price_display"]);
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_matrix_lexer_variable_slice_filter() {
    let mut lexer = Lexer::new(
        "{{ obj_a.owner.first_name|slice:\":1\"|add_str:obj_a.owner.last_name|slice:\":2\" }}",
    );
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::Variable { filters, .. } => {
            assert!(filters.len() >= 2);

            let slice_filters: Vec<_> = filters
                .iter()
                .filter(|filter| filter.name == "slice")
                .collect();
            assert!(slice_filters.len() >= 2);
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_matrix_lexer_for_with_filtered_iterable() {
    let mut lexer = Lexer::new(
        "{% for assignment in dispatch.assignments|get_item:column %}{{ assignment }}{% endfor %}",
    );
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "for");
            assert!(content.contains("dispatch.assignments|get_item:column"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_for_tuple_unpacking_with_items() {
    let mut lexer =
        Lexer::new("{% for material, load_counts in daily_report_entity.load_summaries.items %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "for");
            assert!(content.contains("material, load_counts"));
            assert!(content.contains("daily_report_entity.load_summaries.items"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_widthratio_tag() {
    let mut lexer = Lexer::new("{% widthratio total_billable_hours total_expected_hours 100 %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "widthratio");
            assert!(content.contains("total_billable_hours"));
            assert!(content.contains("total_expected_hours"));
            assert!(content.contains("100"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_firstof_tag() {
    let mut lexer = Lexer::new("{% firstof attribute_value or x_attribute_value as value %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "firstof");
            assert!(content.contains("attribute_value"));
            assert!(content.contains("x_attribute_value"));
            assert!(content.contains("as value"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_user_list_from_content_type() {
    let mut lexer = Lexer::new("{% user_list_from_content_type obj_a as comment_user_list %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "user_list_from_content_type");
            assert!(content.contains("obj_a"));
            assert!(content.contains("as comment_user_list"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_crispy_tag() {
    let mut lexer = Lexer::new("{% crispy comment_form %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "crispy");
            assert!(content.contains("comment_form"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_url_with_many_kwargs_and_as() {
    let mut lexer = Lexer::new(
        "{% url 'fake_ns:view_g' app_id=app_id deployment_id=active_id log_type='BUILD' as build_url %}",
    );
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "url");
            assert!(content.contains("'fake_ns:view_g'"));
            assert!(content.contains("app_id=app_id"));
            assert!(content.contains("deployment_id=active_id"));
            assert!(content.contains("log_type='BUILD'"));
            assert!(content.contains("as build_url"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_url_with_filter_in_kwarg() {
    let mut lexer = Lexer::new("{% url 'fake_ns:view_ag' pk=obj_a.pk|default:0 as form_url %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "url");
            assert!(content.contains("pk=obj_a.pk|default:0"));
            assert!(content.contains("as form_url"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_with_as_syntax_and_filter_chain() {
    let mut lexer = Lexer::new(
        r#"{% with employee.latest_feedback_datetime|date:'F j, Y'|default:"None" as feedback_date %}"#,
    );
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "with");
            assert!(content.contains("employee.latest_feedback_datetime"));
            assert!(content.contains("|date:"));
            assert!(content.contains("|default:"));
            assert!(content.contains("as feedback_date"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_with_string_concatenation_chain() {
    let mut lexer =
        Lexer::new("{% with ''|add:total_count|add:' / '|add:error_count as count_display %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "with");
            assert!(content.contains("|add:total_count"));
            assert!(content.contains("|add:' / '"));
            assert!(content.contains("|add:error_count"));
            assert!(content.contains("as count_display"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_with_addstr_and_cut_chain() {
    let mut lexer =
        Lexer::new("{% with button_id='prefix_'|add:perm_data.app_name|cut:\" \"|lower %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "with");
            assert!(content.contains("|add:perm_data.app_name"));
            assert!(content.contains("|cut:"));
            assert!(content.contains("|lower"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_variable_default_if_none_with_unicode() {
    let mut lexer = Lexer::new(r#"{{ value|default_if_none:"—"|truncatechars:40 }}"#);
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::Variable { filters, .. } => {
            assert_eq!(filters.len(), 2);
            assert_eq!(filters[0].name, "default_if_none");
            assert_eq!(filters[1].name, "truncatechars");
            assert_eq!(filters[1].arguments, vec!["40"]);
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_matrix_lexer_variable_yesno_filter() {
    let mut lexer = Lexer::new(r#"{{ value|yesno:"Yes,No" }}"#);
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::Variable { filters, .. } => {
            assert_eq!(filters.len(), 1);
            assert_eq!(filters[0].name, "yesno");
            assert_eq!(filters[0].arguments, vec!["\"Yes,No\""]);
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_matrix_lexer_variable_floatformat_with_string_arg() {
    let mut lexer = Lexer::new(r#"{{ value|floatformat:"2g" }}"#);
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::Variable { filters, .. } => {
            assert_eq!(filters.len(), 1);
            assert_eq!(filters[0].name, "floatformat");
            assert_eq!(filters[0].arguments, vec!["\"2g\""]);
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_matrix_lexer_if_with_filter_and_comparison() {
    let mut lexer = Lexer::new(
        "{% if perms.app_a.change_model_a and not obj_a.status|in_list:'done,rejected,cancelled' %}",
    );
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "if");
            assert!(content.contains("perms.app_a.change_model_a"));
            assert!(content.contains("and not"));
            assert!(content.contains("|in_list:'done,rejected,cancelled'"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_if_with_multiple_or_conditions() {
    let mut lexer =
        Lexer::new("{% if perms.app_e.view_x or perms.app_e.view_y or perms.app_e.view_z %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "if");
            assert!(content.contains("perms.app_e.view_x"));
            assert!(content.contains("or perms.app_e.view_y"));
            assert!(content.contains("or perms.app_e.view_z"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_if_complex_and_or_chain() {
    let mut lexer = Lexer::new(
        "{% if perms.app_b.delete_model_b or perms.app_c.add_model_c and request.user.id == obj_a.requestor_id and obj_a.phase == 'req' %}",
    );
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "if");
            assert!(content.contains("perms.app_b.delete_model_b"));
            assert!(content.contains("or perms.app_c.add_model_c"));
            assert!(content.contains("request.user.id == obj_a.requestor_id"));
            assert!(content.contains("obj_a.phase == 'req'"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_if_with_emoji_in_string() {
    let mut lexer = Lexer::new("{% if '🏗️ In Progress' in status %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "if");
            assert!(content.contains("🏗️ In Progress"));
            assert!(content.contains("in status"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_if_with_date_filter_comparison() {
    let mut lexer = Lexer::new("{% if issue.due_date|date:'Y-m-d' < today %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "if");
            assert!(content.contains("issue.due_date|date:'Y-m-d'"));
            assert!(content.contains("< today"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_include_with_complex_filter_in_with() {
    let mut lexer = Lexer::new(
        "{% include 'pkg_a/comp_c/elem_f.html' with value=obj_a.notes|default_if_none:\"—\"|truncatechars:40 %}",
    );
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "include");
            assert!(content.contains("'pkg_a/comp_c/elem_f.html'"));
            assert!(content.contains("|default_if_none:"));
            assert!(content.contains("|truncatechars:40"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_include_with_floatformat_intcomma_chain() {
    let mut lexer = Lexer::new(
        "{% include 'pkg_a/comp_c/elem_e.html' with value=obj_a.amount|floatformat:2 suffix=' CUR' %}",
    );
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "include");
            assert!(content.contains("|floatformat:2"));
            assert!(content.contains("suffix=' CUR'"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_load_multiple_libraries() {
    let mut lexer = Lexer::new("{% load crispy_forms_filters static %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "load");
            assert!(content.contains("crispy_forms_filters"));
            assert!(content.contains("static"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_now_with_format_and_as() {
    let mut lexer = Lexer::new(r#"{% now "Y-m-d" as today %}"#);
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "now");
            assert!(content.contains("\"Y-m-d\""));
            assert!(content.contains("as today"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_session_controller_to_json_with_string_key() {
    let mut lexer = Lexer::new("{% session_controller_to_json 'task_list_filter' %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "session_controller_to_json");
            assert!(content.contains("'task_list_filter'"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_with_model_filters_binding() {
    let mut lexer =
        Lexer::new("{% with app_label=obj|model_app_label model_name=obj|model_name %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "with");
            assert!(content.contains("app_label=obj|model_app_label"));
            assert!(content.contains("model_name=obj|model_name"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_with_dashes_and_spaces_to_underscore() {
    let mut lexer =
        Lexer::new("{% with 'display_'|add:key|dashes_and_spaces_to_underscore as key_display %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "with");
            assert!(content.contains("|add:key"));
            assert!(content.contains("|dashes_and_spaces_to_underscore"));
            assert!(content.contains("as key_display"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_matrix_lexer_with_floatformat_intcomma_chain() {
    let mut lexer = Lexer::new("{% with price_display=line_item.price|floatformat:2|intcomma %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "with");
            assert!(content.contains("price_display=line_item.price"));
            assert!(content.contains("|floatformat:2"));
            assert!(content.contains("|intcomma"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}
