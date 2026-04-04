use crate::common::compile_simple;

fn assert_round_trip(input: &str) {
    let output = compile_simple(input);
    let trimmed_input = input.trim();
    let trimmed_output = output.trim();

    assert_eq!(
        trimmed_output, trimmed_input,
        "\n--- ROUND-TRIP FAILURE ---\n  input:  {}\n  output: {}\n",
        trimmed_input, trimmed_output,
    );
}

fn assert_lex_parse_ok(input: &str) {
    let output = compile_simple(input);

    assert!(
        !output.is_empty(),
        "compile_simple returned empty for input: {}",
        input,
    );
}

#[test]
fn test_matrix_battery_variables_and_filters() {
    let cases = [
        r#"{{ variable|default:"" }}"#,
        r#"{{ key_stack|add:'|'|add:key }}"#,
        r#"{{ var|default:''|upper|escape }}"#,
        r#"{{ var|slice:'1:3' }}"#,
        r#"{{ dict.key }}"#,
        r#"{{ list.0 }}"#,
        r#"{{ obj.get_name }}"#,
        r#"{{ very_long_variable_name_with_many_parts }}"#,
        r#"{{ var|f1|f2|f3|f4|f5|f6|f7|f8|f9|f10 }}"#,
        r#"{{ value|date:"Y-m-d H:i:s" }}"#,
        r#"{{ value|truncatechars:40 }}"#,
        r#"{{ value|floatformat:"2g" }}"#,
        r#"{{ value|linebreaksbr }}"#,
        r#"{{ value|default_if_none:"—" }}"#,
        r#"{{ value|default:'N/A' }}"#,
        r#"{{ value|yesno:"Yes,No" }}"#,
        r#"{{ value|cut:'.pdf' }}"#,
        r#"{{ value|title }}"#,
        r#"{{ value|upper }}"#,
        r#"{{ value|lower }}"#,
        r#"{{ value|intcomma }}"#,
        r#"{{ value|default:"Not Sent" }}"#,
        r#"{{ value|default:'Not set' }}"#,
        r#"{{ value|default:'Unassigned' }}"#,
        r#"{{ value|truncatewords:100 }}"#,
        r#"{{ value|date:"M d, Y g:i A" }}"#,
        r#"{{ value|date:'F j, Y' }}"#,
        r#"{{ value|date:'M j, Y' }}"#,
        r#"{{ value|add:' - '|add:other_value }}"#,
        r#"{{ "$"|add:price_display }}"#,
        r#"{{ "$"|add:cost_display }}"#,
        r#"{{ person.first_name|add_str:' '|add_str:person.last_name }}"#,
        r#"{{ state_code|in_list:'new,pending,closed' }}"#,
        r#"{{ record|model_app_label }}"#,
        r#"{{ record|model_name }}"#,
        r#"{{ payload|safe_dict_items }}"#,
        r#"{{ obj_a.owner.first_name|slice:":1"|add_str:obj_a.owner.last_name|slice:":2" }}"#,
        r#"{{ value|default_if_none:"—"|truncatechars:40 }}"#,
        r#"{{ obj_a.amount|floatformat:2 }}"#,
        r#"{{ obj_a.priority|default:"Normal"|upper }}"#,
        r#"{{ obj_a.notes|default_if_none:"—"|truncatechars:40 }}"#,
        r#"{{ obj_a.file_name|cut:'.pdf' }}"#,
        r#"{{ obj_a.created_at|date:"M d, Y g:i A" }}"#,
        r#"{{ obj_a.get_status_display }}"#,
    ];

    for case in &cases {
        assert_round_trip(case);
    }
}

#[test]
fn test_matrix_battery_if_simple_boolean() {
    let cases = [
        "{% if rows %}yes{% endif %}",
        "{% if messages %}yes{% endif %}",
        "{% if tooltip_text %}yes{% endif %}",
        "{% if sort_key %}yes{% endif %}",
        "{% if button_text %}yes{% endif %}",
        "{% if button_href %}yes{% endif %}",
        "{% if href %}yes{% endif %}",
        "{% if label %}yes{% endif %}",
        "{% if title %}yes{% endif %}",
        "{% if value %}yes{% endif %}",
        "{% if name %}yes{% endif %}",
        "{% if form %}yes{% endif %}",
        "{% if query %}yes{% endif %}",
        "{% if queryset %}yes{% endif %}",
        "{% if checked %}yes{% endif %}",
        "{% if disabled %}yes{% endif %}",
        "{% if style %}yes{% endif %}",
        "{% if text %}yes{% endif %}",
        "{% if video %}yes{% endif %}",
        "{% if recurrence %}yes{% endif %}",
        "{% if file_list %}yes{% endif %}",
        "{% if children_list %}yes{% endif %}",
        "{% if comment_list %}yes{% endif %}",
    ];

    for case in &cases {
        assert_lex_parse_ok(case);
    }
}

#[test]
fn test_matrix_battery_if_negated() {
    let cases = [
        "{% if not disable_overlay %}yes{% endif %}",
        "{% if not hide_loading %}yes{% endif %}",
        "{% if not hide_button %}yes{% endif %}",
        "{% if not agreement.is_locked %}yes{% endif %}",
        "{% if not employee.pk %}yes{% endif %}",
        "{% if not forloop.last %}yes{% endif %}",
        "{% if not forloop.first %}yes{% endif %}",
        "{% if not is_child %}yes{% endif %}",
        "{% if not is_loading %}yes{% endif %}",
        "{% if not request.user.is_authenticated %}yes{% endif %}",
    ];

    for case in &cases {
        assert_lex_parse_ok(case);
    }
}

#[test]
fn test_matrix_battery_if_permissions() {
    let cases = [
        "{% if perms.app_a.view_model_a %}yes{% endif %}",
        "{% if perms.app_a.change_model_a %}yes{% endif %}",
        "{% if perms.app_a.delete_model_a %}yes{% endif %}",
        "{% if perms.app_a.add_model_a %}yes{% endif %}",
        "{% if perms.app_a.view_model_a or perms.app_b.view_model_b %}yes{% endif %}",
        "{% if perms.app_a.view_model_a or perms.app_b.view_model_b or perms.app_c.view_model_c %}yes{% endif %}",
        "{% if perms.app_a.change_model_a and not obj_a.status|in_list:'done,rejected,cancelled' %}yes{% endif %}",
        "{% if perms.app_a.change_model_a and obj_a.phase != 'done' %}yes{% endif %}",
        "{% if perms.app_a.change_model_a and obj_a.status|in_list:'req,rev,ipr' %}yes{% endif %}",
        "{% if perms.app_a.change_model_a and obj_a.status == 'req' %}yes{% endif %}",
        "{% if perms.app_a.change_model_a and obj_a.pk %}yes{% endif %}",
        "{% if perms.app_b.delete_model_b or perms.app_c.add_model_c and request.user.id == obj_a.requestor_id and obj_a.phase == 'req' %}yes{% endif %}",
        "{% if perms.app_d.can_manage_entries or request.user == obj_a.user %}yes{% endif %}",
        "{% if perms.app_d.can_approve and obj_a.metric != 0 %}yes{% endif %}",
        "{% if perms.app_e.view_x or perms.app_e.view_y or perms.app_e.view_z %}yes{% endif %}",
        "{% if request.user.is_authenticated %}yes{% endif %}",
        "{% if request.user.is_superuser %}yes{% endif %}",
        "{% if request.user == obj_a.user %}yes{% endif %}",
        "{% if request.user == time_entry.user %}yes{% endif %}",
    ];

    for case in &cases {
        assert_lex_parse_ok(case);
    }
}

#[test]
fn test_matrix_battery_if_status_comparisons() {
    let cases = [
        "{% if obj_a.status == 'pen' %}yes{% endif %}",
        "{% if obj_a.status == 'app' %}yes{% endif %}",
        "{% if obj_a.status == 'rej' %}yes{% endif %}",
        "{% if obj_a.status == 'can' %}yes{% endif %}",
        "{% if obj_a.priority == 'Highest' %}yes{% endif %}",
        "{% if obj_a.priority == 'High' %}yes{% endif %}",
        "{% if obj_a.priority == 'Medium' %}yes{% endif %}",
        "{% if obj_a.priority == 'Low' %}yes{% endif %}",
        "{% if obj_a.stage == StageChoices.IDENTIFY %}yes{% endif %}",
        "{% if obj_a.stage == StageChoices.CONNECT %}yes{% endif %}",
        "{% if obj_a.stage == StageChoices.WON %}yes{% endif %}",
        "{% if obj_a.stage == StageChoices.LOST %}yes{% endif %}",
        "{% if obj_a.relationship == 'hot' %}yes{% endif %}",
        "{% if obj_a.relationship == 'warm' %}yes{% endif %}",
        "{% if obj_a.relationship == 'cold' %}yes{% endif %}",
        "{% if obj_a.life_cycle == LifeCycleChoices.ALPHA %}yes{% endif %}",
        "{% if obj_a.life_cycle == LifeCycleChoices.STABLE %}yes{% endif %}",
        "{% if content_type is None %}yes{% endif %}",
        "{% if value != None %}yes{% endif %}",
        "{% if growth_value >= 0.0 %}yes{% endif %}",
        "{% if growth_value >= 0 %}yes{% endif %}",
        "{% if date < current_date %}yes{% endif %}",
        "{% if issue.due_date|date:'Y-m-d' < today %}yes{% endif %}",
        "{% if issue.due_date|date:'Y-m-d' == today %}yes{% endif %}",
        "{% if forloop.first %}yes{% endif %}",
        "{% if forloop.last %}yes{% endif %}",
        "{% if forloop.first == forloop.last %}yes{% endif %}",
        "{% if i == paginated_list.paginator.ELLIPSIS %}yes{% endif %}",
        "{% if paginated_list.number == i %}yes{% endif %}",
        "{% if comment.user == request.user %}yes{% endif %}",
        "{% if comment.is_edited %}yes{% endif %}",
        "{% if permit.phase == 'app' %}yes{% endif %}",
        "{% if permit.phase == 'iss' %}yes{% endif %}",
        "{% if permit.general.signature_method == 'draw' %}yes{% endif %}",
        "{% if permit.general.permit_type == 'type_a' %}yes{% endif %}",
        "{% if permission_level|lower == 'view' %}yes{% endif %}",
        "{% if permit.applicant.applicant_type|is_equal:'own' %}yes{% endif %}",
        "{% if permit.applicant.applicant_type|in_list:'con,oth' %}yes{% endif %}",
        "{% if row.pk == field.current_version_id %}yes{% endif %}",
        "{% if current_external_portal_user.role == 'adm' %}yes{% endif %}",
    ];

    for case in &cases {
        assert_lex_parse_ok(case);
    }
}

#[test]
fn test_matrix_battery_if_emoji_strings() {
    let cases = [
        "{% if '🎆 Done' in status %}yes{% endif %}",
        "{% if '🤔 Waiting Information' in status %}yes{% endif %}",
        "{% if '🔎 Code Review' in status %}yes{% endif %}",
        "{% if '👷‍♂️ Ready' in status %}yes{% endif %}",
        "{% if '🏗️ In Progress' in status %}yes{% endif %}",
    ];

    for case in &cases {
        assert_lex_parse_ok(case);
    }
}

#[test]
fn test_matrix_battery_for_loops() {
    let cases = [
        "{% for breadcrumb in breadcrumbs %}{{ breadcrumb }}{% endfor %}",
        "{% for deal in deals %}{{ deal }}{% endfor %}",
        "{% for project in projects %}{{ project }}{% endfor %}",
        "{% for partner in partners %}{{ partner }}{% endfor %}",
        "{% for agreement in agreements %}{{ agreement }}{% endfor %}",
        "{% for group in group_list %}{{ group }}{% endfor %}",
        "{% for user in user_list %}{{ user }}{% endfor %}",
        "{% for skill in skills %}{{ skill }}{% endfor %}",
        "{% for meeting in meetings %}{{ meeting }}{% endfor %}",
        "{% for feedback in feedback_list %}{{ feedback }}{% endfor %}",
        "{% for contact in contacts %}{{ contact }}{% endfor %}",
        "{% for file in file_list %}{{ file }}{% endfor %}",
        "{% for comment in comment_list %}{{ comment }}{% endfor %}",
        "{% for comment in children_list %}{{ comment }}{% endfor %}",
        "{% for row in rows %}{{ row }}{% endfor %}",
        "{% for cell in row.cells %}{{ cell }}{% endfor %}",
        "{% for activity in activity_log %}{{ activity }}{% endfor %}",
        "{% for entry in entries %}{{ entry }}{% endfor %}",
        "{% for work_order in work_orders %}{{ work_order }}{% endfor %}",
        "{% for task in tasks %}{{ task }}{% endfor %}",
        "{% for role in roles %}{{ role }}{% endfor %}",
        "{% for issue in issues %}{{ issue }}{% endfor %}",
        "{% for employee in employees %}{{ employee }}{% endfor %}",
        "{% for employee in crew.employees %}{{ employee }}{% endfor %}",
        "{% for employee in assignment.employees %}{{ employee }}{% endfor %}",
        "{% for project in client.projects.all %}{{ project }}{% endfor %}",
        "{% for work_order in day.work_orders %}{{ work_order }}{% endfor %}",
        "{% for video in videos %}{{ video }}{% endfor %}",
        "{% for lesson in lessons %}{{ lesson }}{% endfor %}",
    ];

    for case in &cases {
        assert_lex_parse_ok(case);
    }
}

#[test]
fn test_matrix_battery_for_tuple_unpacking() {
    let cases = [
        "{% for key, val in params.choices %}{{ key }}{% endfor %}",
        "{% for deliverable, price in metrics.deliverables_prices.items %}{{ deliverable }}{% endfor %}",
        "{% for user, data in reporting_data.items %}{{ user }}{% endfor %}",
        "{% for material, load_counts in daily_report_entity.load_summaries.items %}{{ material }}{% endfor %}",
    ];

    for case in &cases {
        assert_lex_parse_ok(case);
    }
}

#[test]
fn test_matrix_battery_for_with_filtered_iterable() {
    let cases = [
        "{% for request in request_calendar|get_item:date %}{{ request }}{% endfor %}",
        "{% for assignment in dispatch.assignments|get_item:column %}{{ assignment }}{% endfor %}",
    ];

    for case in &cases {
        assert_lex_parse_ok(case);
    }
}

#[test]
fn test_matrix_battery_with_simple_bindings() {
    let cases = [
        "{% with divider_text='GroupB' %}{{ divider_text }}{% endwith %}",
        "{% with icon_size='size-3' %}{{ icon_size }}{% endwith %}",
        "{% with badge_text='Low' %}{{ badge_text }}{% endwith %}",
        "{% with badge_text='Active' %}{{ badge_text }}{% endwith %}",
        "{% with badge_text='Complete' %}{{ badge_text }}{% endwith %}",
        "{% with badge_text='In Progress' %}{{ badge_text }}{% endwith %}",
        "{% with badge_text='Private' %}{{ badge_text }}{% endwith %}",
        "{% with badge_text=status %}{{ badge_text }}{% endwith %}",
        "{% with endpoint=table_rows_url %}{{ endpoint }}{% endwith %}",
        "{% with value=partner.has_active_engagement %}{{ value }}{% endwith %}",
        "{% with value=contact.is_active %}{{ value }}{% endwith %}",
        "{% with partner_name=obj_a.partner.name %}{{ partner_name }}{% endwith %}",
        "{% with permit_entity as entity_a %}{{ entity_a }}{% endwith %}",
        "{% with pipeline_item.template as template %}{{ template }}{% endwith %}",
        "{% with obj=binding.obj field=binding.field %}{{ obj }}{% endwith %}",
        "{% with employee_state=feedback.employee_state %}{{ employee_state }}{% endwith %}",
    ];

    for case in &cases {
        assert_lex_parse_ok(case);
    }
}

#[test]
fn test_matrix_battery_with_filter_chain_bindings() {
    let cases = [
        r#"{% with badge_text=value|yesno:"Yes,No" %}{{ badge_text }}{% endwith %}"#,
        r#"{% with badge_text=obj_a.get_stage_display %}{{ badge_text }}{% endwith %}"#,
        r#"{% with badge_text=days_since|add_str:' Days' as badge_text %}{{ badge_text }}{% endwith %}"#,
        r#"{% with badge_text=obj_a.owner.first_name|slice:":1"|add_str:obj_a.owner.last_name|slice:":2" %}{{ badge_text }}{% endwith %}"#,
        r#"{% with 'mailto:'|add_str:contact.email as email_url %}{{ email_url }}{% endwith %}"#,
        r#"{% with 'tel:'|add_str:contact.phone|default:"" as phone_url %}{{ phone_url }}{% endwith %}"#,
        r#"{% with title|lower|cut:" " as tab_id %}{{ tab_id }}{% endwith %}"#,
        r#"{% with pk_str=inventory.pk|stringformat:"d" %}{{ pk_str }}{% endwith %}"#,
        r#"{% with remaining=total|subtract:ready_count %}{{ remaining }}{% endwith %}"#,
        r#"{% with scrap_display=record.scrap_percentage|floatformat:2 %}{{ scrap_display }}{% endwith %}"#,
        r#"{% with employee.latest_feedback_datetime|date:'F j, Y'|default:"None" as feedback_date %}{{ feedback_date }}{% endwith %}"#,
        r#"{% with deal.value|floatformat:"2g" as deal_value %}{{ deal_value }}{% endwith %}"#,
        r#"{% with price_display=line_item.price|floatformat:2|intcomma %}{{ price_display }}{% endwith %}"#,
        r#"{% with cost_display=line_item.total_cost|floatformat:2|intcomma %}{{ cost_display }}{% endwith %}"#,
        r#"{% with navigation_url|add:'?return_url='|add:return_url as navigation_with_return_url %}{{ navigation_with_return_url }}{% endwith %}"#,
        r#"{% with navigation_url|add:'?return_url='|add:request.path as navigation_with_return_url %}{{ navigation_with_return_url }}{% endwith %}"#,
        r#"{% with ''|add:total_count|add:' / '|add:error_count as count_display %}{{ count_display }}{% endwith %}"#,
        r#"{% with button_id='prefix_'|addstr:req_a.pk %}{{ button_id }}{% endwith %}"#,
        r#"{% with button_id='prefix_'|add:perm_data.app_name|cut:" "|lower %}{{ button_id }}{% endwith %}"#,
        r#"{% with app_label=obj|model_app_label model_name=obj|model_name %}{{ app_label }}{% endwith %}"#,
        r#"{% with 'display_'|add:key|dashes_and_spaces_to_underscore as key_display %}{{ key_display }}{% endwith %}"#,
        r#"{% with status_update_url|addstr:"?tab=2" as status_update_url %}{{ status_update_url }}{% endwith %}"#,
    ];

    for case in &cases {
        assert_lex_parse_ok(case);
    }
}

#[test]
fn test_matrix_battery_url_tags() {
    let cases = [
        "{% url 'ns_a:ns_b:view_a' as url_a %}",
        "{% url 'ns_a:ns_b:view_b' pk=obj_a.pk as url_b %}",
        "{% url 'ns_a:ns_b:view_c' pk=obj_a.pk|default:0 %}",
        "{% url 'ns_a:ns_b:view_e' parent_pk=obj_a.parent_id pk=obj_a.pk as url_d %}",
        "{% url 'ns_a:ns_b:view_f' external_access_key=request.bridge.access_key as url_e %}",
        "{% url 'ns_a:ns_b:view_g' app_id=app_id deployment_id=active_id log_type='RUN' as run_url %}",
        "{% url 'ns_a:ns_b:view_g' app_id=app_id deployment_id=active_id log_type='BUILD' as build_url %}",
        "{% url 'ns_a:ns_b:view_g' app_id=app_id deployment_id=active_id log_type='DEPLOY' as deploy_url %}",
        "{% url 'ns_a:ns_b:view_i' pk='MARKER_ID' %}",
        "{% url 'ns_a:ns_b:view_k' field_version_pk='VERSION_ID' pk=item.pk %}",
        "{% url 'ns_a:ns_b:view_l' pk=0 order=0 %}",
        "{% url 'ns_a:ns_b:view_n' uidb64=uid token=token %}",
        "{% url 'ns_a:ns_b:view_o' collection_pk=collection.pk as create_url %}",
        "{% url 'ns_a:ns_b:view_p' collection_pk=collection_pk|default:0 %}",
        "{% url 'ns_a:ns_b:view_s' app_name=perm_data.app_name pk=group.pk %}",
        "{% url 'ns_a:ns_b:view_t' bool_field_name=feature_access.bool_field_name as update_url %}",
        "{% url 'ns_a:ns_b:view_v' reorder_pk=req_a.pk pk=0 as reorder_url %}",
        "{% url 'ns_a:ns_b:view_w' phase='can' pk=req_a.pk as phase_url %}",
        "{% url 'ns_a:ns_b:view_x' project_pk=obj_a.pk user_type=UserTypeChoices.LEAD as lead_endpoint %}",
        "{% url 'ns_a:ns_b:view_ab' pk=obj_a.pk as delete_url %}",
        "{% url 'ns_a:ns_b:view_ae' as list_url %}",
        "{% url 'ns_a:ns_b:view_af' pk=0 as new_url %}",
        "{% url 'ns_a:ns_b:view_ag' pk=obj_a.pk|default:0 as form_url %}",
        r#"{% url "ns_a:ns_b:view_ao" %}"#,
        r#"{% url "ns_a:ns_b:view_ap" pk=999 %}"#,
        r#"{% url "ns_a:ns_b:view_aq" pk=0 %}"#,
        r#"{% url "ns_a:ns_b:view_ar" pk='MARKER_ID' %}"#,
    ];

    for case in &cases {
        assert_round_trip(case);
    }
}

#[test]
fn test_matrix_battery_custom_tags() {
    let cases = [
        r#"{% now "F jS, Y" %}"#,
        r#"{% now "Y-m-d" as today %}"#,
        "{% get_elided_page_range paginated_list as page_range %}",
        "{% session_controller_to_json filter_session.session_key %}",
        "{% session_controller_to_json 'task_list_filter' %}",
        "{% user_list_from_content_type obj_a as comment_user_list %}",
        "{% widthratio total_non_billable_hours total_expected_hours 100 %}",
        "{% widthratio total_missing_hours total_expected_hours 100 %}",
        "{% widthratio total_billable_hours total_expected_hours 100 %}",
        "{% string_to_font_scale value char_limit %}",
        "{% form_label_to_name label %}",
        "{% generate_id as response_message_id %}",
        "{% content_type_url 'ns_a:form' obj comment_pk=0 obj_pk=obj.pk return_url=request.get_full_path %}",
        r#"{% content_type_url "ns_a:user_list" obj %}"#,
        "{% comment_form related_obj request.get_full_path request.user user_list=comment_user_list as comment_form %}",
        "{% comment_form related_obj request.get_full_path request.user parent=comment user_list=comment_user_list as comment_form %}",
        "{% grouping_url edit_url_name object_pk=model.pk pk=comment.pk as comment_edit_url %}",
        "{% grouping_url delete_url_name object_pk=model.pk pk=comment.pk as comment_delete_url %}",
        "{% form_url 'ns_a:ns_b:view_a' pk=time_entry.pk user_pk=time_entry.user.pk as edit_url %}",
        "{% django_messages_to_json messages %}",
        "{% crispy form %}",
        "{% crispy comment_form %}",
        "{% csrf_token %}",
        "{% debug %}",
    ];

    for case in &cases {
        assert_round_trip(case);
    }
}

#[test]
fn test_matrix_battery_load_tags() {
    let cases = [
        "{% load static %}",
        "{% load i18n %}",
        "{% load static django_glue %}",
        "{% load i18n static %}",
        "{% load crispy_forms_filters static %}",
    ];

    for case in &cases {
        assert_round_trip(case);
    }
}

#[test]
fn test_matrix_battery_include_simple_with_args() {
    let cases = [
        "{% include 'pkg_a/comp_b/item_c.html' with arg_icon='cls cls-x1' arg_text='LabelA' x_arg_action=\"fnAlpha()\" %}",
        "{% include 'pkg_a/comp_b/item_e.html' with arg_text='LabelD' arg_class='u-size-md' %}",
        "{% include 'pkg_a/comp_b/item_l.html' with arg_text='LabelI' %}",
        "{% include 'pkg_a/comp_b/item_m.html' with arg_text='LabelJ' %}",
        "{% include 'pkg_a/comp_b/item_r.html' with arg_text='LabelO' %}",
        "{% include 'pkg_a/comp_b/item_s.html' with arg_text='LabelP' arg_icon='cls cls-trash' %}",
        "{% include 'pkg_a/comp_b/item_w.html' with arg_text='Layer A' arg_class='btn-layer-a' %}",
        "{% include 'pkg_a/comp_b/item_az.html' with arg_text='LabelAN' arg_class='btn' %}",
        "{% include 'pkg_a/comp_c/elem_u.html' with subheading='GroupA' %}",
        "{% include 'pkg_a/comp_c/elem_v.html' with help_text='*Required field text here' %}",
        "{% include 'pkg_a/comp_c/elem_y.html' with title='HeadingA' subtitle='SubheadingA' %}",
        "{% include 'pkg_a/comp_c/elem_ab.html' with url=the_url %}",
        "{% include 'pkg_a/comp_c/elem_ad.html' with queryset=file_list %}",
        "{% include 'pkg_a/comp_c/elem_ae.html' with child_endpoint=child_endpoint %}",
        "{% include 'pkg_a/comp_c/elem_ah.html' with icon_size='fs-4' %}",
        "{% include 'pkg_a/comp_c/elem_ai.html' with scroll_height='400px' %}",
        "{% include 'pkg_a/comp_c/elem_ak.html' with glue_field='search' %}",
        "{% include 'pkg_a/comp_c/elem_aq.html' with glue_field='email' %}",
        "{% include 'pkg_a/comp_c/elem_at.html' with glue_field='status' %}",
        "{% include 'pkg_a/comp_c/elem_s.html' with input_value_class='border-bottom' %}",
        "{% include 'pkg_a/comp_c/elem_ac.html' with card_color='bg-light' border_color='border-soft' text_color='text-muted' button_type='btn-primary' %}",
    ];

    for case in &cases {
        assert_round_trip(case);
    }
}

#[test]
fn test_matrix_battery_include_with_filter_chains() {
    let cases = [
        "{% include 'pkg_a/comp_c/elem_a.html' with label=obj_a.rel_b.name color='tone-1' dismissible=True %}",
        "{% include 'pkg_a/comp_c/elem_b.html' with text=obj_a.state|title count=obj_a.rel_c.count css_class='u1 u2' %}",
        r#"{% include 'pkg_a/comp_c/elem_c.html' with item_url=obj_a.get_absolute_url item_label=obj_a.name|default:"ValueA" %}"#,
        "{% include 'pkg_a/comp_c/elem_e.html' with value=obj_a.amount|floatformat:2 suffix=' CUR' %}",
        r#"{% include 'pkg_a/comp_c/elem_f.html' with value=obj_a.notes|default_if_none:"—"|truncatechars:40 %}"#,
        "{% include 'pkg_a/comp_c/elem_g.html' with image_url=user_a.profile.avatar.url|default:'/static/img/default.png' alt_text=user_a.get_full_name %}",
        "{% include 'pkg_a/comp_c/elem_h.html' with href=obj_a.file.url target='_blank' link_text=obj_a.file_name|cut:'.pdf' %}",
        r#"{% include 'pkg_a/comp_c/elem_i.html' with badge_text=value|yesno:"Yes,No" %}"#,
        r#"{% include 'pkg_a/comp_c/elem_k.html' with badge_text=obj_a.priority|default:"Normal"|upper %}"#,
        r#"{% include 'pkg_a/comp_c/elem_l.html' with badge_text=obj_a.owner.first_name|slice:":1"|add_str:obj_a.owner.last_name|slice:":2" %}"#,
        "{% include 'pkg_a/comp_c/elem_n.html' with attribute_title='FieldB' attribute_value=obj_a.description|linebreaksbr %}",
        r#"{% include 'pkg_a/comp_c/elem_o.html' with attribute_title='FieldC' attribute_value_prefix='$' attribute_value=obj_a.total|floatformat:'2g' %}"#,
        r#"{% include 'pkg_a/comp_c/elem_p.html' with attribute_title='FieldD' attribute_value=obj_a.created_at|date:"M d, Y g:i A" %}"#,
        "{% include 'pkg_a/comp_c/elem_q.html' with attribute_title='FieldE' attribute_value=obj_a.contact.email attribute_href=email_url %}",
        "{% include 'pkg_a/comp_c/elem_t.html' with value=obj_a.signature_name value_class='cursive' %}",
        "{% include 'pkg_a/comp_c/elem_aj.html' with scroll_height=scroll_height|default:'300px' %}",
        "{% include 'pkg_a/comp_c/elem_db.html' with glue_field='image' accept='.png, .jpg' %}",
        "{% include 'pkg_a/comp_c/elem_dd.html' with initial_value=item.feedback|default:'' x_model_name='feedback' %}",
    ];

    for case in &cases {
        assert_round_trip(case);
    }
}

#[test]
fn test_matrix_battery_include_with_alpine_actions() {
    let cases = [
        "{% include 'pkg_a/comp_b/item_g.html' with arg_text='+ Item' x_arg_action='showCreateModal()' %}",
        "{% include 'pkg_a/comp_b/item_k.html' with x_arg_action='remove_row(index)' arg_icon='cls cls-trash' %}",
        "{% include 'pkg_a/comp_b/item_n.html' with x_arg_action='submit_value(true)' arg_text='LabelK' %}",
        r#"{% include 'pkg_a/comp_b/item_p.html' with arg_text='LabelM' x_arg_action="delete_item()" %}"#,
        "{% include 'pkg_a/comp_b/item_t.html' with arg_icon='cls cls-trash' x_arg_action='showDeleteModal()' %}",
        r#"{% include 'pkg_a/comp_b/item_u.html' with arg_icon='cls cls-trash' x_arg_action="showDeleteFileModal()" %}"#,
        "{% include 'pkg_a/comp_b/item_ad.html' with arg_text='LabelT' arg_class='btn' x_arg_action='await save_config()' %}",
        "{% include 'pkg_a/comp_b/item_aj.html' with arg_text='LabelZ' arg_icon='cls cls-copy' x_arg_action='await copy_secret()' arg_class='btn-success' %}",
        r#"{% include 'pkg_a/comp_b/item_aw.html' with arg_text="LabelAK" x_arg_action="shouldDelete = true; await submit();" %}"#,
        r#"{% include 'pkg_a/comp_b/item_ax.html' with arg_text="LabelAL" x_arg_action="await deleteEntity()" %}"#,
        r#"{% include 'pkg_a/comp_b/item_ay.html' with arg_text="LabelAM" x_arg_action="await deleteRow()" %}"#,
        "{% include 'pkg_a/comp_b/item_ah.html' with arg_text='LabelX' x_arg_action='showMobileNav = false;' arg_class='col-12 btn-success btn-sm' %}",
    ];

    for case in &cases {
        assert_round_trip(case);
    }
}

#[test]
fn test_matrix_battery_include_with_glue_model_fields() {
    let cases = [
        "{% include 'pkg_a/comp_c/elem_ca.html' with glue_model_field='obj_a.quantity' %}",
        "{% include 'pkg_a/comp_c/elem_cb.html' with glue_model_field='obj_a.price' %}",
        "{% include 'pkg_a/comp_c/elem_cc.html' with glue_model_field='obj_a.duration' %}",
        "{% include 'pkg_a/comp_c/elem_cf.html' with glue_model_field='obj_a.is_private' %}",
        "{% include 'pkg_a/comp_c/elem_cg.html' with glue_model_field='obj_a.general_format' %}",
        "{% include 'pkg_a/comp_c/elem_ch.html' with glue_model_field='obj_a.city' %}",
        "{% include 'pkg_a/comp_c/elem_ci.html' with glue_model_field='obj_a.country' %}",
        "{% include 'pkg_a/comp_c/elem_ck.html' with glue_model_field='obj_a.website' %}",
        "{% include 'pkg_a/comp_c/elem_cn.html' with glue_model_field='obj_a.maintainer' %}",
        "{% include 'pkg_a/comp_c/elem_co.html' with glue_model_field='obj_a.finder' %}",
        "{% include 'pkg_a/comp_c/elem_cp.html' with glue_model_field='obj_a.closer' %}",
    ];

    for case in &cases {
        assert_round_trip(case);
    }
}

#[test]
fn test_matrix_battery_firstof_tags() {
    let cases = [
        "{% firstof attribute_value or x_attribute_value as value %}",
        r#"{% firstof headline fallback_headline "Fallback headline" as chosen_headline %}"#,
    ];

    for case in &cases {
        assert_round_trip(case);
    }
}

#[test]
fn test_matrix_battery_alpine_django_mixed() {
    let cases = [
        r#"<div x-data="{ open: false }">Sidebar with {{ variable|default:"default" }}</div>"#,
        r#"<div x-data="{ init() { this.value = '{{ variable|default:"" }}'; }, controller: new Controller('{% url 'ns_a:ns_b:view_a' %}') }">Content</div>"#,
        r#"<div @click="$dispatch('navigate', { url: '{{ qs }}' })">Next Page</div>"#,
        r#"<div x-text="message">{{ message|escape }}</div>"#,
        r#"<script>var x = 1;</script>"#,
    ];

    for case in &cases {
        assert_lex_parse_ok(case);
    }
}

#[test]
fn test_matrix_battery_body_tags() {
    let cases = [
        r#"{% cache 500 sidebar %}<div x-data="{ open: false }">Sidebar with {{ variable|default:"default" }}</div>{% endcache %}"#,
        r#"{% localize on %}<div x-text="message">{{ message|escape }}</div>{% endlocalize %}"#,
        r#"{% timezone "America/New_York" %}<time>{{ date|date:"Y-m-d H:i:s" }}</time>{% endtimezone %}"#,
        r#"{% autoescape off %}{{ raw }}{% endautoescape %}"#,
        r#"{% blocktranslate %}Text{% endblocktranslate %}"#,
    ];

    for case in &cases {
        assert_lex_parse_ok(case);
    }
}
