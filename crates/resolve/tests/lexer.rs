use compiler::lexer::{Lexer, Token};

#[test]
fn test_lexer_simple_variable() {
    let mut lexer = Lexer::new("{{ variable }}");
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(tokens.len(), 2);
    match &tokens[0] {
        Token::Variable {
            expression,
            filters,
            raw,
        } => {
            assert_eq!(*expression, "variable");
            assert!(filters.is_empty());
            assert_eq!(*raw, "{{ variable }}");
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_lexer_variable_with_filter() {
    let mut lexer = Lexer::new("{{ variable|default:\"\" }}");
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(tokens.len(), 2);
    match &tokens[0] {
        Token::Variable {
            expression,
            filters,
            raw,
        } => {
            assert_eq!(*expression, "variable|default:\"\"");
            assert_eq!(filters.len(), 1);
            assert_eq!(filters[0].name, "default");
            assert_eq!(filters[0].arguments, vec!["\"\""]);
            assert_eq!(*raw, "{{ variable|default:\"\" }}");
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_lexer_chained_filters() {
    let mut lexer = Lexer::new("{{ key_stack|add:'|'|add:key }}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::Variable { filters, .. } => {
            assert_eq!(filters.len(), 2);
            assert_eq!(filters[0].name, "add");
            assert_eq!(filters[0].arguments, vec!["'|'"]);
            assert_eq!(filters[1].name, "add");
            assert_eq!(filters[1].arguments, vec!["key"]);
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_lexer_variable_with_multiple_filters() {
    let mut lexer = Lexer::new("{{ var|default:''|upper|escape }}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::Variable { filters, .. } => {
            assert_eq!(filters.len(), 3);
            assert_eq!(filters[0].name, "default");
            assert_eq!(filters[1].name, "upper");
            assert_eq!(filters[2].name, "escape");
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_lexer_variable_with_complex_filter_args() {
    let mut lexer = Lexer::new("{{ var|slice:'1:3' }}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::Variable { filters, .. } => {
            assert_eq!(filters.len(), 1);
            assert_eq!(filters[0].name, "slice");
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_lexer_variable_with_dict_lookup() {
    let mut lexer = Lexer::new("{{ dict.key }}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::Variable { expression, .. } => {
            assert_eq!(*expression, "dict.key");
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_lexer_variable_with_list_index() {
    let mut lexer = Lexer::new("{{ list.0 }}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::Variable { expression, .. } => {
            assert_eq!(*expression, "list.0");
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_lexer_variable_with_method_call() {
    let mut lexer = Lexer::new("{{ obj.get_name }}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::Variable { expression, .. } => {
            assert_eq!(*expression, "obj.get_name");
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_lexer_very_long_variable_name() {
    let mut lexer = Lexer::new("{{ very_long_variable_name_with_many_parts }}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::Variable { expression, .. } => {
            assert!(expression.contains("very_long_variable_name_with_many_parts"));
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_lexer_many_filters() {
    let mut lexer = Lexer::new("{{ var|f1|f2|f3|f4|f5|f6|f7|f8|f9|f10 }}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::Variable { filters, .. } => {
            assert_eq!(filters.len(), 10);
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_lexer_block_start() {
    let mut lexer = Lexer::new("{% if condition %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, raw } => {
            assert_eq!(*tag, "if");
            assert!(content.contains("condition"));
            assert_eq!(*raw, "{% if condition %}");
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_block_end() {
    let mut lexer = Lexer::new("{% endif %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockEnd { tag, .. } => {
            assert_eq!(*tag, "if");
        }
        _ => panic!("Expected BlockEnd token"),
    }
}

#[test]
fn test_lexer_if_endif_tokens() {
    let template = "{% if x %}hello{% endif %}";
    let mut lexer = Lexer::new(template);
    let tokens = lexer.tokenize().unwrap();

    assert!(tokens.len() >= 4);

    let has_if = tokens
        .iter()
        .any(|t| matches!(t, Token::BlockStart { tag, .. } if *tag == "if"));
    let has_endif = tokens
        .iter()
        .any(|t| matches!(t, Token::BlockEnd { tag, .. } if *tag == "if"));

    assert!(has_if);
    assert!(has_endif);
}

#[test]
fn test_lexer_extends_tag() {
    let mut lexer = Lexer::new("{% extends 'base.html' %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "extends");
            assert!(content.contains("base.html"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_include_tag() {
    let mut lexer = Lexer::new("{% include 'component.html' with var=value %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "include");
            assert!(content.contains("component.html"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_block_tag() {
    let mut lexer = Lexer::new("{% block content %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "block");
            assert!(content.contains("content"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_endblock_tag() {
    let mut lexer = Lexer::new("{% endblock %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockEnd { tag, .. } => {
            assert_eq!(*tag, "block");
        }
        _ => panic!("Expected BlockEnd token"),
    }
}

#[test]
fn test_lexer_named_endblock_tag() {
    let mut lexer = Lexer::new("{% endblock content %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockEnd { tag, .. } => {
            assert_eq!(*tag, "block");
        }
        _ => panic!("Expected BlockEnd token"),
    }
}

#[test]
fn test_lexer_named_endblock_with_title() {
    let mut lexer = Lexer::new("{% endblock title %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockEnd { tag, .. } => {
            assert_eq!(*tag, "block");
        }
        _ => panic!("Expected BlockEnd token"),
    }
}

#[test]
fn test_lexer_load_tag() {
    let mut lexer = Lexer::new("{% load static django_glue %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "load");
            assert!(content.contains("static"));
            assert!(content.contains("django_glue"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_url_tag() {
    let mut lexer = Lexer::new("{% url 'home' as home_url %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "url");
            assert!(content.contains("home"));
            assert!(content.contains("as"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_static_tag() {
    let mut lexer = Lexer::new("{% static 'css/style.css' %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "static");
            assert!(content.contains("css/style.css"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_csrf_token() {
    let mut lexer = Lexer::new("{% csrf_token %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, .. } => {
            assert_eq!(*tag, "csrf_token");
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_for_tag() {
    let mut lexer = Lexer::new("{% for item in items %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "for");
            assert!(content.contains("item"));
            assert!(content.contains("items"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_for_tuple_unpacking() {
    let mut lexer = Lexer::new("{% for key, value in dict.items %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "for");
            assert!(content.contains("key, value"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_empty_tag() {
    let mut lexer = Lexer::new("{% empty %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, .. } => {
            assert_eq!(*tag, "empty");
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_with_tag() {
    let mut lexer = Lexer::new("{% with total=price|add:tax %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "with");
            assert!(content.contains("total"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_ifchanged_tag() {
    let mut lexer = Lexer::new("{% ifchanged %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, .. } => {
            assert_eq!(*tag, "ifchanged");
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_filter_tag() {
    let mut lexer = Lexer::new("{% filter upper %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "filter");
            assert!(content.contains("upper"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_now_tag() {
    let mut lexer = Lexer::new("{% now \"Y-m-d\" %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "now");
            assert!(content.contains("Y-m-d"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_cycle_tag() {
    let mut lexer = Lexer::new("{% cycle 'a' 'b' 'c' as colors %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "cycle");
            assert!(content.contains("'a'"));
            assert!(content.contains("as"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_firstof_tag() {
    let mut lexer = Lexer::new("{% firstof var1 var2 var3 %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "firstof");
            assert!(content.contains("var1"));
            assert!(content.contains("var2"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_widthratio_tag() {
    let mut lexer = Lexer::new("{% widthratio value max 100 %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "widthratio");
            assert!(content.contains("value"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_trans_tag() {
    let mut lexer = Lexer::new("{% trans \"Hello\" %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "trans");
            assert!(content.contains("Hello"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_blocktranslate_tag() {
    let mut lexer = Lexer::new("{% blocktranslate %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, .. } => {
            assert_eq!(*tag, "blocktranslate");
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_language_tag() {
    let mut lexer = Lexer::new("{% language 'en' %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "language");
            assert!(content.contains("'en'"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_verbatim_tag() {
    let mut lexer = Lexer::new("{% verbatim %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, .. } => {
            assert_eq!(*tag, "verbatim");
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_templatetag_tag() {
    let mut lexer = Lexer::new("{% templatetag \"openblock\" %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "templatetag");
            assert!(content.contains("openblock"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_autoescape_tag() {
    let mut lexer = Lexer::new("{% autoescape off %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, content, .. } => {
            assert_eq!(*tag, "autoescape");
            assert!(content.contains("off"));
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_comment_tag() {
    let mut lexer = Lexer::new("{% comment %}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::BlockStart { tag, .. } => {
            assert_eq!(*tag, "comment");
        }
        _ => panic!("Expected BlockStart token"),
    }
}

#[test]
fn test_lexer_cache_tag() {
    let mut lexer = Lexer::new("{% cache 500 sidebar %}");
    let tokens = lexer.tokenize().unwrap();
    assert!(matches!(&tokens[0], Token::BlockStart { tag, .. } if *tag == "cache"));
}

#[test]
fn test_lexer_localize_tag() {
    let mut lexer = Lexer::new("{% localize on %}");
    let tokens = lexer.tokenize().unwrap();
    assert!(matches!(&tokens[0], Token::BlockStart { tag, .. } if *tag == "localize"));
}

#[test]
fn test_lexer_localtime_tag() {
    let mut lexer = Lexer::new("{% localtime on %}");
    let tokens = lexer.tokenize().unwrap();
    assert!(matches!(&tokens[0], Token::BlockStart { tag, .. } if *tag == "localtime"));
}

#[test]
fn test_lexer_spaceless_tag() {
    let mut lexer = Lexer::new("{% spaceless %}");
    let tokens = lexer.tokenize().unwrap();
    assert!(matches!(&tokens[0], Token::BlockStart { tag, .. } if *tag == "spaceless"));
}

#[test]
fn test_lexer_timezone_tag() {
    let mut lexer = Lexer::new(r#"{% timezone "UTC" %}"#);
    let tokens = lexer.tokenize().unwrap();
    assert!(matches!(&tokens[0], Token::BlockStart { tag, .. } if *tag == "timezone"));
}

#[test]
fn test_lexer_utc_tag() {
    let mut lexer = Lexer::new("{% utc %}");
    let tokens = lexer.tokenize().unwrap();
    assert!(matches!(&tokens[0], Token::BlockStart { tag, .. } if *tag == "utc"));
}

#[test]
fn test_lexer_debug_tag() {
    let mut lexer = Lexer::new("{% debug %}");
    let tokens = lexer.tokenize().unwrap();
    assert!(matches!(&tokens[0], Token::BlockStart { tag, .. } if *tag == "debug"));
}

#[test]
fn test_lexer_get_static_prefix_tag() {
    let mut lexer = Lexer::new("{% get_static_prefix %}");
    let tokens = lexer.tokenize().unwrap();
    assert!(matches!(&tokens[0], Token::BlockStart { tag, .. } if *tag == "get_static_prefix"));
}

#[test]
fn test_lexer_get_media_prefix_tag() {
    let mut lexer = Lexer::new("{% get_media_prefix %}");
    let tokens = lexer.tokenize().unwrap();
    assert!(matches!(&tokens[0], Token::BlockStart { tag, .. } if *tag == "get_media_prefix"));
}

#[test]
fn test_lexer_resetcycle_tag() {
    let mut lexer = Lexer::new("{% resetcycle mycycle %}");
    let tokens = lexer.tokenize().unwrap();
    assert!(matches!(&tokens[0], Token::BlockStart { tag, .. } if *tag == "resetcycle"));
}

#[test]
fn test_lexer_lorem_tag() {
    let mut lexer = Lexer::new("{% lorem 5 p %}");
    let tokens = lexer.tokenize().unwrap();
    assert!(matches!(&tokens[0], Token::BlockStart { tag, .. } if *tag == "lorem"));
}

#[test]
fn test_lexer_querystring_tag() {
    let mut lexer = Lexer::new("{% querystring foo=bar %}");
    let tokens = lexer.tokenize().unwrap();
    assert!(matches!(&tokens[0], Token::BlockStart { tag, .. } if *tag == "querystring"));
}

#[test]
fn test_lexer_translate_tag() {
    let mut lexer = Lexer::new(r#"{% translate "Hello" %}"#);
    let tokens = lexer.tokenize().unwrap();
    assert!(matches!(&tokens[0], Token::BlockStart { tag, .. } if *tag == "translate"));
}

#[test]
fn test_lexer_plural_tag() {
    let mut lexer = Lexer::new("{% plural %}");
    let tokens = lexer.tokenize().unwrap();
    assert!(matches!(&tokens[0], Token::BlockStart { tag, .. } if *tag == "plural"));
}

#[test]
fn test_lexer_regroup_tag() {
    let mut lexer = Lexer::new("{% regroup people by gender as gender_list %}");
    let tokens = lexer.tokenize().unwrap();
    assert!(matches!(&tokens[0], Token::BlockStart { tag, .. } if *tag == "regroup"));
}

#[test]
fn test_lexer_inline_comment() {
    let mut lexer = Lexer::new("{# This is a comment #}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::Comment(content) => {
            assert!(content.contains("comment"));
        }
        _ => panic!("Expected Comment token"),
    }
}

#[test]
fn test_lexer_block_comment() {
    let mut lexer = Lexer::new("{% comment %}Block comment{% endcomment %}");
    let tokens = lexer.tokenize().unwrap();

    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::BlockStart { tag, .. } if *tag == "comment"))
    );
}

#[test]
fn test_lexer_html_passthrough() {
    let mut lexer = Lexer::new("<div class=\"test\">Hello</div>");
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(tokens.len(), 2);
    match &tokens[0] {
        Token::Text(content) => {
            assert_eq!(*content, "<div class=\"test\">Hello</div>");
        }
        _ => panic!("Expected Text token"),
    }
}

#[test]
fn test_lexer_text_content() {
    let mut lexer = Lexer::new("<div class=\"test\">Hello World</div>");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
    assert!(matches!(tokens[0], Token::Text(_)));
}

#[test]
fn test_lexer_text_with_html() {
    let mut lexer = Lexer::new("<p>This is <strong>bold</strong> text</p>");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_text_with_special_chars() {
    let mut lexer = Lexer::new("<div>&lt;script&gt;alert('xss')&lt;/script&gt;</div>");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_text_multiline() {
    let mut lexer = Lexer::new("Line 1\nLine 2\nLine 3");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_alpine_xdata_passthrough() {
    let template = r#"<div x-data="{ value: 1 }" class="test">Content</div>"#;
    let mut lexer = Lexer::new(template);
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(tokens.len(), 2);
    match &tokens[0] {
        Token::Text(content) => {
            assert!(content.contains("x-data"));
            assert!(content.contains("{ value: 1 }"));
        }
        _ => panic!("Expected Text token"),
    }
}

#[test]
fn test_lexer_alpine_multiline_xdata() {
    let template = r#"<div x-data="{
    init() {
        this.value = 'test';
    }
}">Content</div>"#;
    let mut lexer = Lexer::new(template);
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(tokens.len(), 2);
    match &tokens[0] {
        Token::Text(content) => {
            assert!(content.contains("x-data"));
            assert!(content.contains("init()"));
        }
        _ => panic!("Expected Text token"),
    }
}

#[test]
fn test_lexer_alpine_data_simple() {
    let mut lexer = Lexer::new("<div x-data=\"{ open: false }\">");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_alpine_data_with_django() {
    let mut lexer = Lexer::new("<div x-data=\"{ value: '{{ var }}' }\">");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_alpine_complex_x_data() {
    let template = r#"<div x-data="{
    init() {
        this.value = '{{ var }}';
        this.load();
    },
    async load() {
        const res = await fetch('{% url "api" %}');
        this.data = await res.json();
    }
}">"#;

    let mut lexer = Lexer::new(template);
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_alpine_template_x_for() {
    let mut lexer = Lexer::new("<template x-for=\"item in items\">");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_alpine_template_x_if() {
    let mut lexer = Lexer::new("<template x-if=\"condition\">");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_alpine_bind() {
    let mut lexer = Lexer::new("<div :class=\"{ active: isOpen }\">");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_alpine_on() {
    let mut lexer = Lexer::new("<div @click=\"handleClick()\">");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_alpine_model() {
    let mut lexer = Lexer::new("<input x-model=\"value\">");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_alpine_show() {
    let mut lexer = Lexer::new("<div x-show=\"isVisible\">");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_alpine_text_directive() {
    let mut lexer = Lexer::new(r#"<div x-text="message"></div>"#);
    let tokens = lexer.tokenize().unwrap();

    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::Text(text) if text.contains("x-text")))
    );
}

#[test]
fn test_lexer_alpine_html_directive() {
    let mut lexer = Lexer::new(r#"<div x-html="content"></div>"#);
    let tokens = lexer.tokenize().unwrap();

    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::Text(text) if text.contains("x-html")))
    );
}

#[test]
fn test_lexer_alpine_teleport() {
    let mut lexer = Lexer::new(r#"<div x-teleport="body"></div>"#);
    let tokens = lexer.tokenize().unwrap();

    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::Text(text) if text.contains("x-teleport")))
    );
}

#[test]
fn test_lexer_alpine_id() {
    let mut lexer = Lexer::new(r#"<div x-id="['modal']"></div>"#);
    let tokens = lexer.tokenize().unwrap();

    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::Text(text) if text.contains("x-id")))
    );
}

#[test]
fn test_lexer_alpine_transition() {
    let mut lexer = Lexer::new("<div x-transition:enter=\"transition\">");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_alpine_cloak() {
    let mut lexer = Lexer::new("<div x-cloak>");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_alpine_ref() {
    let mut lexer = Lexer::new("<div x-ref=\"myRef\">");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_alpine_ignore() {
    let mut lexer = Lexer::new("<div x-ignore>");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_alpine_trap() {
    let mut lexer = Lexer::new("<div x-trap=\"isOpen\">");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_alpine_spread() {
    let mut lexer = Lexer::new("<div x-spread=\"data\">");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_alpine_effect() {
    let mut lexer = Lexer::new("<div x-effect=\"console.log()\">");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_alpine_init() {
    let mut lexer = Lexer::new("<div x-init=\"init()\">");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_mixed_django_and_alpine() {
    let template = r#"<div x-data="{ open: false }" {% if perms.edit %}class="editable"{% endif %}>
    {{ content }}
</div>"#;

    let mut lexer = Lexer::new(template);
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_empty_input() {
    let mut lexer = Lexer::new("");
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0], Token::Eof));
}

#[test]
fn test_lexer_whitespace_only() {
    let mut lexer = Lexer::new("   \n\t  ");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_only_variable() {
    let mut lexer = Lexer::new("{{ var }}");
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(tokens.len(), 2);
}

#[test]
fn test_lexer_only_block() {
    let mut lexer = Lexer::new("{% if %}{% endif %}");
    let tokens = lexer.tokenize().unwrap();

    assert!(tokens.len() >= 2);
}

#[test]
fn test_lexer_nested_braces_in_string() {
    let mut lexer = Lexer::new("{{ var|default:'{}' }}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::Variable { expression, .. } => {
            assert!(expression.contains("{}"));
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_lexer_nested_quotes() {
    let mut lexer = Lexer::new("{{ var|default:\"'nested'\" }}");
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::Variable { expression, .. } => {
            assert!(expression.contains("'nested'"));
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_lexer_escaped_quotes() {
    let mut lexer = Lexer::new("{{ var|default:\"it\\'s\" }}");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_unicode_characters() {
    let mut lexer = Lexer::new("{{ greeting }}");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_emoji_in_text() {
    let mut lexer = Lexer::new("<div>\u{1F389} Hello \u{1F38A}</div>");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_unclosed_variable() {
    let mut lexer = Lexer::new("{{ unclosed");
    let result = lexer.tokenize();

    assert!(result.is_err());
}

#[test]
fn test_lexer_unclosed_block() {
    let mut lexer = Lexer::new("{% unclosed");
    let result = lexer.tokenize();

    assert!(result.is_err());
}

#[test]
fn test_lexer_unclosed_comment() {
    let mut lexer = Lexer::new("{# unclosed");
    let result = lexer.tokenize();

    assert!(result.is_err());
}

#[test]
fn test_lexer_malformed_tag() {
    let mut lexer = Lexer::new("{% malformed tag %}");
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_round_trip() {
    let template = r#"<div x-data="{ value: {{ var|default:\"\" }} }" class="test" {% if cond %}active{% endif %}>
    <template x-for="item in items">
        <span>{{ item.name|upper }}</span>
    </template>
</div>"#;
    let mut lexer = Lexer::new(template);
    let tokens = lexer.tokenize().unwrap();

    let mut reconstructed = String::new();
    for token in &tokens {
        match token {
            Token::Text(content) => reconstructed.push_str(content),
            Token::Variable { raw, .. } => reconstructed.push_str(raw),
            Token::BlockStart { raw, .. } => reconstructed.push_str(raw),
            Token::BlockEnd { raw, .. } => reconstructed.push_str(raw),
            Token::Comment(content) => {
                reconstructed.push_str(&format!("{{# {} #}}", content));
            }
            Token::Eof => {}
        }
    }

    assert_eq!(reconstructed, template);
}

#[test]
fn test_lexer_verbatim_flexible_whitespace() {
    let template = "{% verbatim %}{{ not_parsed }}{%  endverbatim  %}";
    let mut lexer = Lexer::new(template);
    let tokens = lexer.tokenize().unwrap();

    let has_verbatim_start = tokens
        .iter()
        .any(|t| matches!(t, Token::BlockStart { tag, .. } if *tag == "verbatim"));
    assert!(has_verbatim_start);

    let has_literal_text = tokens
        .iter()
        .any(|t| matches!(t, Token::Text(c) if c.contains("{{ not_parsed }}")));
    assert!(has_literal_text);
}

#[test]
fn test_lexer_filter_argument_colon_inside_quotes() {
    let mut lexer = Lexer::new(r#"{{ event.start|date:"H:i:s" }}"#);
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::Variable {
            expression,
            filters,
            raw,
        } => {
            assert_eq!(filters.len(), 1);
            assert_eq!(filters[0].name, "date");
            assert_eq!(filters[0].arguments.len(), 1);
            assert_eq!(filters[0].arguments[0], r#""H:i:s""#);

            assert_eq!(*raw, r#"{{ event.start|date:"H:i:s" }}"#,);

            let base = expression.split('|').next().unwrap().trim();
            assert_eq!(base, "event.start");
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_lexer_filter_argument_multiple_colons_in_quoted_string() {
    let mut lexer = Lexer::new(r#"{{ timestamp|date:"Y-m-d H:i:s.u" }}"#);
    let tokens = lexer.tokenize().unwrap();

    match &tokens[0] {
        Token::Variable { filters, .. } => {
            assert_eq!(filters.len(), 1);
            assert_eq!(filters[0].name, "date");
            assert_eq!(filters[0].arguments.len(), 1);

            let arg = filters[0].arguments[0];
            assert!(
                arg.contains("H:i:s"),
                "Colons inside quotes must not split the argument. Got: {}",
                arg,
            );
        }
        _ => panic!("Expected Variable token"),
    }
}

#[test]
fn test_lexer_empty_variable_expression() {
    let mut lexer = Lexer::new("{{  }}");
    let tokens = lexer.tokenize().unwrap();

    let variable = tokens.iter().find(|t| matches!(t, Token::Variable { .. }));

    match variable {
        Some(Token::Variable {
            expression,
            filters,
            ..
        }) => {
            assert!(
                expression.is_empty(),
                "Empty variable expression should trim to empty string. Got: '{}'",
                expression,
            );
            assert!(
                filters.is_empty(),
                "Empty expression should produce no filters",
            );
        }
        _ => {
            assert!(
                tokens.iter().any(|t| matches!(t, Token::Variable { .. })),
                "Lexer should produce a Variable token for '{{{{  }}}}', got: {:?}",
                tokens,
            );
        }
    }
}

#[test]
fn test_lexer_empty_block_tag() {
    let mut lexer = Lexer::new("{%  %}");
    let tokens = lexer.tokenize().unwrap();

    let block = tokens
        .iter()
        .find(|t| matches!(t, Token::BlockStart { .. } | Token::BlockEnd { .. }));

    match block {
        Some(Token::BlockStart { tag, content, .. }) => {
            assert!(
                tag.is_empty(),
                "Empty block should have empty tag. Got: '{}'",
                tag,
            );
            assert!(
                content.is_empty(),
                "Empty block should have empty content. Got: '{}'",
                content,
            );
        }
        Some(Token::BlockEnd { tag, .. }) => {
            assert!(
                tag.is_empty(),
                "Empty block parsed as BlockEnd should have empty tag. Got: '{}'",
                tag,
            );
        }
        _ => {
            panic!(
                "Lexer should produce a block token for '{{%  %}}', got: {:?}",
                tokens,
            );
        }
    }
}

#[test]
fn test_lexer_verbatim_with_fake_endverbatim_trailing_content() {
    let template = "{% verbatim %}raw {{ var }} {% endverbatim extra %}still raw{% endverbatim %}";
    let mut lexer = Lexer::new(template);
    let tokens = lexer.tokenize().unwrap();

    let has_verbatim_start = tokens
        .iter()
        .any(|t| matches!(t, Token::BlockStart { tag, .. } if *tag == "verbatim"));
    assert!(has_verbatim_start);

    let text_tokens: Vec<&str> = tokens
        .iter()
        .filter_map(|t| match t {
            Token::Text(c) => Some(*c),
            _ => None,
        })
        .collect();

    let combined = text_tokens.join("");

    assert!(
        combined.contains("raw {{ var }}"),
        "Raw Django syntax inside verbatim must be preserved as text. Got: '{}'",
        combined,
    );

    assert!(
        combined.contains("{% endverbatim extra %}") || combined.contains("still raw"),
        "Fake endverbatim with trailing content should not terminate the verbatim block, \
         or if it does, 'still raw' should appear as text. Got: '{}'",
        combined,
    );
}

#[test]
fn test_lexer_adjacent_delimiters_no_whitespace() {
    let template = "{{ a }}{{ b }}{% if c %}{{ d }}{% endif %}";
    let mut lexer = Lexer::new(template);
    let tokens = lexer.tokenize().unwrap();

    let variable_count = tokens
        .iter()
        .filter(|t| matches!(t, Token::Variable { .. }))
        .count();

    assert_eq!(
        variable_count, 3,
        "Should produce exactly three Variable tokens for a, b, d. Got: {}",
        variable_count,
    );

    let if_start = tokens
        .iter()
        .find(|t| matches!(t, Token::BlockStart { tag, .. } if *tag == "if"));
    assert!(if_start.is_some(), "Should produce a BlockStart for 'if'",);

    let if_end = tokens
        .iter()
        .find(|t| matches!(t, Token::BlockEnd { tag, .. } if *tag == "if"));
    assert!(if_end.is_some(), "Should produce a BlockEnd for 'endif'",);

    let text_between = tokens
        .iter()
        .filter(|t| matches!(t, Token::Text(_)))
        .count();

    assert_eq!(
        text_between, 0,
        "No text tokens should exist between adjacent delimiters. Got: {}",
        text_between,
    );

    let mut reconstructed = String::new();
    for token in &tokens {
        match token {
            Token::Text(content) => reconstructed.push_str(content),
            Token::Variable { raw, .. } => reconstructed.push_str(raw),
            Token::BlockStart { raw, .. } => reconstructed.push_str(raw),
            Token::BlockEnd { raw, .. } => reconstructed.push_str(raw),
            Token::Comment(content) => {
                reconstructed.push_str(&format!("{{# {} #}}", content));
            }
            Token::Eof => {}
        }
    }

    assert_eq!(
        reconstructed, template,
        "Round-trip must reconstruct the original template",
    );
}
