use resolve::validator::{alpine, filters};

#[test]
fn test_alpine_core_directives() {
    let directives = vec![
        "x-data",
        "x-init",
        "x-model",
        "x-show",
        "x-if",
        "x-for",
        "x-effect",
        "x-ref",
        "x-cloak",
        "x-transition",
        "x-bind",
        "x-on",
        "x-component",
        "x-trap",
        "x-prevent",
        "x-ignore",
        "x-modelable",
        "x-text",
        "x-html",
        "x-teleport",
        "x-id",
    ];

    for directive in directives {
        assert!(
            alpine::validate_directive(directive).is_ok(),
            "Directive '{}' should be valid",
            directive
        );
    }
}

#[test]
fn test_alpine_shorthand_directives() {
    let directives = vec![
        ":class",
        ":style",
        ":id",
        ":disabled",
        ":href",
        "@click",
        "@submit",
        "@input",
    ];

    for directive in directives {
        assert!(
            alpine::validate_directive(directive).is_ok(),
            "Directive '{}' should be valid",
            directive
        );
    }
}

#[test]
fn test_alpine_plugin_directives() {
    let directives = vec![
        "x-intersect",
        "x-intersect:enter",
        "x-intersect:leave",
        "x-collapse",
        "x-mask",
        "x-mask:dynamic",
        "x-sort",
        "x-sort:item",
        "x-sort:group",
        "x-anchor",
    ];

    for directive in directives {
        assert!(
            alpine::validate_directive(directive).is_ok(),
            "Directive '{}' should be valid",
            directive
        );
    }
}

#[test]
fn test_alpine_transition_directives() {
    let directives = vec![
        "x-transition:enter",
        "x-transition:leave",
        "x-transition:enter-start",
        "x-transition:enter-end",
        "x-transition:leave-start",
        "x-transition:leave-end",
    ];

    for directive in directives {
        assert!(
            alpine::validate_directive(directive).is_ok(),
            "Directive '{}' should be valid",
            directive
        );
    }
}

#[test]
fn test_alpine_bind_variants() {
    let directives = vec![
        "x-bind:class",
        "x-bind:style",
        "x-bind:id",
        ":class",
        ":style",
        ":id",
    ];

    for directive in directives {
        assert!(
            alpine::validate_directive(directive).is_ok(),
            "Directive '{}' should be valid",
            directive
        );
    }
}

#[test]
fn test_alpine_on_variants() {
    let directives = vec!["x-on:click", "x-on:submit", "@click", "@submit"];

    for directive in directives {
        assert!(
            alpine::validate_directive(directive).is_ok(),
            "Directive '{}' should be valid",
            directive
        );
    }
}

#[test]
fn test_alpine_magic_properties() {
    let properties = vec![
        "$el",
        "$refs",
        "$store",
        "$watch",
        "$dispatch",
        "$nextTick",
        "$root",
        "$data",
        "$id",
        "$persist",
        "$money",
    ];

    for prop in properties {
        assert!(
            alpine::validate_magic_property(prop).is_ok(),
            "Property '{}' should be valid",
            prop
        );
    }
}

#[test]
fn test_alpine_event_modifiers() {
    let modifiers = vec![
        ".prevent",
        ".stop",
        ".outside",
        ".window",
        ".document",
        ".once",
        ".debounce",
        ".throttle",
        ".self",
        ".camel",
        ".dot",
        ".passive",
        ".capture",
    ];

    for modifier in modifiers {
        let errors = alpine::validate_modifiers(modifier);
        assert!(
            errors.is_empty(),
            "Modifier '{}' should be valid, got errors: {:?}",
            modifier,
            errors
        );
    }
}

#[test]
fn test_alpine_keyboard_modifiers() {
    let modifiers = vec![
        ".shift", ".ctrl", ".alt", ".meta", ".enter", ".escape", ".tab", ".space", ".up", ".down",
        ".left", ".right",
    ];

    for modifier in modifiers {
        let errors = alpine::validate_modifiers(modifier);
        assert!(
            errors.is_empty(),
            "Modifier '{}' should be valid, got errors: {:?}",
            modifier,
            errors
        );
    }
}

#[test]
fn test_alpine_model_modifiers() {
    let modifiers = vec![".lazy", ".number", ".debounce", ".throttle", ".fill"];

    for modifier in modifiers {
        let errors = alpine::validate_modifiers(modifier);
        assert!(
            errors.is_empty(),
            "Modifier '{}' should be valid, got errors: {:?}",
            modifier,
            errors
        );
    }
}

#[test]
fn test_alpine_transition_modifiers() {
    let modifiers = vec![".opacity", ".scale"];

    for modifier in modifiers {
        let errors = alpine::validate_modifiers(modifier);
        assert!(
            errors.is_empty(),
            "Modifier '{}' should be valid, got errors: {:?}",
            modifier,
            errors
        );
    }
}

#[test]
fn test_alpine_multiple_modifiers() {
    let errors = alpine::validate_modifiers(".prevent.stop.once");
    assert!(
        errors.is_empty(),
        "Multiple modifiers should be valid, got: {:?}",
        errors
    );
}

#[test]
fn test_alpine_invalid_modifier() {
    let errors = alpine::validate_modifiers(".invalid");
    assert!(!errors.is_empty(), "Invalid modifier should produce error");
}

#[test]
fn test_filter_registry_contains_expected() {
    let all_filters = filters::all();
    assert!(all_filters.contains("default"));
    assert!(all_filters.contains("upper"));
    assert!(all_filters.contains("length"));
    assert!(all_filters.contains("date"));
    assert!(all_filters.contains("json_script"));
    assert!(all_filters.contains("pluralize"));
    assert!(all_filters.contains("intcomma"));
}

#[test]
fn test_all_string_filters() {
    let string_filters = vec![
        "addslashes",
        "capfirst",
        "center",
        "cut",
        "escape",
        "escapejs",
        "force_escape",
        "iriencode",
        "linebreaks",
        "linebreaksbr",
        "linenumbers",
        "ljust",
        "lower",
        "make_list",
        "phone2numeric",
        "rjust",
        "safe",
        "safeseq",
        "slugify",
        "stringformat",
        "striptags",
        "title",
        "truncatechars",
        "truncatechars_html",
        "truncatewords",
        "truncatewords_html",
        "upper",
        "urlencode",
        "urlize",
        "urlizetrunc",
        "wordcount",
        "wordwrap",
    ];

    for filter in string_filters {
        assert!(
            filters::validate_filter(filter).is_ok(),
            "Filter '{}' should be valid",
            filter
        );
    }
}

#[test]
fn test_all_numeric_filters() {
    let numeric_filters = vec![
        "add",
        "divisibleby",
        "filesizeformat",
        "floatformat",
        "get_digit",
    ];

    for filter in numeric_filters {
        assert!(
            filters::validate_filter(filter).is_ok(),
            "Filter '{}' should be valid",
            filter
        );
    }
}

#[test]
fn test_all_list_dict_filters() {
    let list_dict_filters = vec![
        "dictsort",
        "dictsortreversed",
        "first",
        "join",
        "last",
        "length",
        "length_is",
        "random",
        "slice",
        "unordered_list",
    ];

    for filter in list_dict_filters {
        assert!(
            filters::validate_filter(filter).is_ok(),
            "Filter '{}' should be valid",
            filter
        );
    }
}

#[test]
fn test_all_date_time_filters() {
    let date_filters = vec!["date", "time", "timesince", "timeuntil"];

    for filter in date_filters {
        assert!(
            filters::validate_filter(filter).is_ok(),
            "Filter '{}' should be valid",
            filter
        );
    }
}

#[test]
fn test_all_logic_default_filters() {
    let logic_filters = vec!["default", "default_if_none", "yesno"];

    for filter in logic_filters {
        assert!(
            filters::validate_filter(filter).is_ok(),
            "Filter '{}' should be valid",
            filter
        );
    }
}

#[test]
fn test_all_encoding_serialization_filters() {
    let encoding_filters = vec!["pprint", "json_script"];

    for filter in encoding_filters {
        assert!(
            filters::validate_filter(filter).is_ok(),
            "Filter '{}' should be valid",
            filter
        );
    }
}

#[test]
fn test_pluralize_filter() {
    assert!(filters::validate_filter("pluralize").is_ok());
}

#[test]
fn test_all_humanize_filters() {
    let humanize_filters = vec![
        "apnumber",
        "intcomma",
        "intword",
        "naturalday",
        "naturaltime",
        "ordinal",
    ];

    for filter in humanize_filters {
        assert!(
            filters::validate_filter(filter).is_ok(),
            "Filter '{}' should be valid",
            filter
        );
    }
}

#[test]
fn test_invalid_filter() {
    assert!(filters::validate_filter("nonexistent_filter").is_err());
}

#[test]
fn test_spire_ai_chat_filters() {
    assert!(
        filters::validate_filter("render_markdown").is_ok(),
        "Filter 'render_markdown' should be valid"
    );
}

#[test]
fn test_spire_humanize_filters() {
    let spire_filters = vec!["humanize_duration_simple", "humanize_duration"];

    for filter in spire_filters {
        assert!(
            filters::validate_filter(filter).is_ok(),
            "Spire filter '{}' should be valid",
            filter
        );
    }
}

#[test]
fn test_spire_json_filters() {
    assert!(
        filters::validate_filter("to_json").is_ok(),
        "Filter 'to_json' should be valid"
    );
}

#[test]
fn test_spire_model_filters() {
    let spire_filters = vec!["model_app_label", "model_name"];

    for filter in spire_filters {
        assert!(
            filters::validate_filter(filter).is_ok(),
            "Spire filter '{}' should be valid",
            filter
        );
    }
}

#[test]
fn test_spire_core_filters() {
    let spire_filters = vec![
        "add_str",
        "safe_dict_items",
        "in_list",
        "index",
        "is_path",
        "not_in_list",
        "to_snake_case",
    ];

    for filter in spire_filters {
        assert!(
            filters::validate_filter(filter).is_ok(),
            "Spire filter '{}' should be valid",
            filter
        );
    }
}

#[test]
fn test_spire_string_formatting_filters() {
    let spire_filters = vec![
        "dashes_to_underscore",
        "spaces_to_underscore",
        "dashes_and_spaces_to_underscore",
        "underscores_to_spaces",
    ];

    for filter in spire_filters {
        assert!(
            filters::validate_filter(filter).is_ok(),
            "Spire filter '{}' should be valid",
            filter
        );
    }
}

#[test]
fn test_spire_variable_type_filters() {
    let spire_filters = vec![
        "is_dict",
        "is_not_dict",
        "is_list",
        "is_not_list",
        "is_list_or_tuple",
        "is_not_list_or_tuple",
        "is_tuple",
        "is_not_tuple",
    ];

    for filter in spire_filters {
        assert!(
            filters::validate_filter(filter).is_ok(),
            "Spire filter '{}' should be valid",
            filter
        );
    }
}

#[test]
fn test_spire_knowledge_filters() {
    assert!(
        filters::validate_filter("format_to_html").is_ok(),
        "Filter 'format_to_html' should be valid"
    );
}

#[test]
fn test_spire_simple_tags_are_not_filters() {
    let simple_tags = vec![
        "check_permission",
        "comment_form",
        "user_list_from_content_type",
        "help_button",
        "pagination_url",
        "get_elided_page_range",
        "session_controller_to_json",
        "django_messages_to_json",
        "content_type_url",
        "generate_id",
        "query_param_url",
    ];

    for tag in simple_tags {
        assert!(
            filters::validate_filter(tag).is_err(),
            "Simple tag '{}' should not be a filter",
            tag
        );
    }
}

#[test]
fn test_alpine_chained_modifiers_valid_and_invalid_mixed() {
    let errors = alpine::validate_modifiers(".prevent.nonexistent.stop");

    assert_eq!(
        errors.len(),
        1,
        "Exactly one error should be reported for the single invalid modifier. Got: {:?}",
        errors,
    );

    let error_msg = &errors[0];
    assert!(
        error_msg.contains("nonexistent"),
        "Error should identify the invalid modifier 'nonexistent'. Got: '{}'",
        error_msg,
    );
}

#[test]
fn test_alpine_chained_modifiers_multiple_invalid() {
    let errors = alpine::validate_modifiers(".prevent.bogus.stop.fake.once");

    assert_eq!(
        errors.len(),
        2,
        "Exactly two errors for 'bogus' and 'fake'. Got: {:?}",
        errors,
    );

    let combined = errors.join(" ");
    assert!(
        combined.contains("bogus"),
        "Errors should mention 'bogus'. Got: '{}'",
        combined,
    );
    assert!(
        combined.contains("fake"),
        "Errors should mention 'fake'. Got: '{}'",
        combined,
    );
}

#[test]
fn test_alpine_chained_modifiers_all_valid_returns_empty() {
    let errors = alpine::validate_modifiers(".prevent.stop.once.capture");

    assert!(
        errors.is_empty(),
        "All valid modifiers should produce no errors. Got: {:?}",
        errors,
    );
}

#[test]
#[should_panic(expected = "content must not be empty")]
fn test_alpine_validate_modifiers_empty_string_panics() {
    let _ = alpine::validate_modifiers("");
}
