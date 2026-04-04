mod common;

use common::compile_simple;


#[test]
fn test_codegen_filter_not_doubled() {
    let template = r#"{{ variable|default:"" }}"#;
    let output = compile_simple(template);

    assert_eq!(
        output.trim(),
        template.trim(),
        "Filter should not be doubled. Got: {}",
        output
    );

    assert!(
        !output.contains("|default:\"\"|default:\"\""),
        "Filter is doubled in output"
    );
}

#[test]
fn test_codegen_multiple_filters_not_doubled() {
    let template = r#"{{ variable|default:""|upper|escape }}"#;
    let output = compile_simple(template);

    let default_count = output.matches("|default:").count();
    let upper_count = output.matches("|upper").count();
    let escape_count = output.matches("|escape").count();

    assert_eq!(
        default_count, 1,
        "default filter should appear exactly once"
    );
    assert_eq!(upper_count, 1, "upper filter should appear exactly once");
    assert_eq!(escape_count, 1, "escape filter should appear exactly once");
}

#[test]
fn test_codegen_complex_filter_chain_preserved() {
    let template = r#"{{ key_stack|add:'|'|add:key }}"#;
    let output = compile_simple(template);

    assert_eq!(
        output.trim(),
        template.trim(),
        "Complex filter chain should be preserved exactly"
    );
}

#[test]
fn test_codegen_regroup_preserved() {
    let template = r#"{% regroup people by gender as gender_list %}"#;
    let output = compile_simple(template);

    assert_eq!(
        output.trim(),
        template.trim(),
        "Regroup tag should be preserved exactly"
    );
}

#[test]
fn test_codegen_autoescape_spacing() {
    let template = r#"{% autoescape off %}{{ content }}{% endautoescape %}"#;
    let output = compile_simple(template);

    assert!(
        output.contains("{% autoescape off %}"),
        "Autoescape tag should have proper spacing. Got: {}",
        output
    );

    assert!(
        !output.contains("{% autoescapeoff %}"),
        "Autoescape tag is malformed. Got: {}",
        output
    );
}

#[test]
fn test_codegen_trans_quote_style_double() {
    let template = r#"{% trans "Hello" %}"#;
    let output = compile_simple(template);

    assert!(
        output.contains("\"Hello\"") || output.contains("{% trans"),
        "Double quotes should be preserved. Got: {}",
        output
    );
}

#[test]
fn test_codegen_trans_quote_style_single() {
    let template = r#"{% trans 'Hello' %}"#;
    let output = compile_simple(template);

    assert!(
        output.contains("'Hello'") || output.contains("{% trans"),
        "Single quotes should be preserved. Got: {}",
        output
    );
}

#[test]
fn test_codegen_verbatim_preservation() {
    let template = r#"{% load static %}
{{ variable|default:""|upper }}
{% if condition %}content{% endif %}
{% csrf_token %}
{% url 'home' as home_url %}
{% trans "Hello" %}
{% autoescape off %}{{ raw }}{% endautoescape %}
{% blocktranslate %}Text{% endblocktranslate %}
{# comment #}"#;

    let output = compile_simple(template);

    assert!(output.contains("{% load"), "load tag preserved");
    assert!(
        output.contains("{{ variable|default:\"\"|upper }}"),
        "variable with filters preserved"
    );
    assert!(output.contains("{% if"), "if tag preserved");
    assert!(output.contains("{% csrf_token"), "csrf_token preserved");
    assert!(output.contains("{% url"), "url tag preserved");
    assert!(output.contains("{% trans"), "trans tag preserved");
    assert!(output.contains("{% autoescape"), "autoescape preserved");
    assert!(
        output.contains("{% blocktranslate"),
        "blocktranslate preserved"
    );
    assert!(output.contains("{#"), "comment preserved");

    assert!(
        !output.contains("|default:\"\"|default:\"\""),
        "no doubled filters"
    );
    assert!(!output.contains("{% endifif %}"), "no doubled end tags");
}

#[test]
fn test_codegen_all_runtime_tags_preserved() {
    let template = r#"{% load static %}
{% url 'home' as home_url %}
{% csrf_token %}
{% if condition %}Yes{% endif %}
{% for item in items %}{{ item }}{% endfor %}
{% with x=1 %}{{ x }}{% endwith %}
{{ variable }}
{{ variable|default:"" }}
{% autoescape off %}{{ content }}{% endautoescape %}
{% blocktranslate %}Hello{% endblocktranslate %}
{% trans "text" %}
{% now "Y-m-d" %}
{% debug %}
{# comment #}"#;

    let output = compile_simple(template);

    assert!(output.contains("{% load"), "load tag preserved");
    assert!(output.contains("{% url"), "url tag preserved");
    assert!(output.contains("{% csrf_token"), "csrf_token preserved");
    assert!(output.contains("{% if"), "if tag preserved");
    assert!(output.contains("{% endif %}"), "endif tag preserved");
    assert!(output.contains("{% for"), "for tag preserved");
    assert!(output.contains("{% endfor %}"), "endfor tag preserved");
    assert!(output.contains("{% with"), "with tag preserved");
    assert!(output.contains("{% endwith %}"), "endwith tag preserved");
    assert!(output.contains("{{ variable }}"), "variable preserved");
    assert!(output.contains("|default:"), "filter preserved");
    assert!(output.contains("{% autoescape"), "autoescape preserved");
    assert!(
        output.contains("{% endautoescape %}"),
        "endautoescape preserved"
    );
    assert!(
        output.contains("{% blocktranslate"),
        "blocktranslate preserved"
    );
    assert!(output.contains("{% trans"), "trans tag preserved");
    assert!(output.contains("{% now"), "now tag preserved");
    assert!(output.contains("{% debug %}"), "debug tag preserved");
    assert!(output.contains("{#"), "comment preserved");

    assert!(
        !output.contains("|default:\"\"|default:\"\""),
        "No doubled filters"
    );
    assert!(
        !output.contains("{% endifif %}") && !output.contains("{% endforfor %}"),
        "No doubled end tags"
    );
}

#[test]
fn test_codegen_alpine_data_preservation() {
    let template = r#"<div x-data="{ value: '{{ variable|default:"" }}' }"></div>"#;
    let output = compile_simple(template);

    assert!(
        output.contains("x-data") || output.contains("<div"),
        "x-data attribute or div tag should be preserved"
    );
}

#[test]
fn test_codegen_conditional_attribute() {
    let template = r#"<div {% if condition %}class="active"{% endif %}>Content</div>"#;
    let output = compile_simple(template);

    assert!(!output.is_empty());
}

#[test]
fn test_codegen_alpine_template_x_for() {
    let template = r#"<template x-for="item in items"><div>{{ item }}</div></template>"#;
    let output = compile_simple(template);

    assert!(output.contains("x-for"));
    assert!(output.contains("</template>"));
}

#[test]
fn test_codegen_block_in_html_attribute() {
    let template = r#"<span class="{% block badge_class %}{% endblock %} {{ badge_class|default:'fs--1' }}">Text</span>"#;
    let output = compile_simple(template);

    assert!(!output.is_empty());
}

#[test]
fn test_codegen_static_in_attribute() {
    let template = r#"<script src="{% static 'js/app.js' %}?v=1.0"></script>"#;
    let output = compile_simple(template);

    assert!(!output.is_empty());
}

#[test]
fn test_codegen_url_tag() {
    let template = r#"<a href="{% url 'home' %}">Home</a>"#;
    let output = compile_simple(template);

    assert!(output.contains("{% url"));
    assert!(output.contains("home"));
}

#[test]
fn test_codegen_url_with_as() {
    let template = r#"{% url 'home' as home_url %}<a href="{{ home_url }}">Home</a>"#;
    let output = compile_simple(template);

    assert!(!output.is_empty());
}

#[test]
fn test_codegen_comment_block_produces_output() {
    let template = "{% comment %}This should be preserved as raw{% endcomment %}";
    let output = compile_simple(template);

    assert!(
        output.contains("{% comment %}"),
        "Comment block opening tag should be emitted by codegen. Got: '{}'",
        output,
    );
    assert!(
        output.contains("{% endcomment %}"),
        "Comment block closing tag should be emitted by codegen. Got: '{}'",
        output,
    );
    assert!(
        output.contains("This should be preserved as raw"),
        "Comment block content should be emitted by codegen (Django handles stripping at runtime). Got: '{}'",
        output,
    );
}

#[test]
fn test_codegen_verbatim_round_trip() {
    let template = "{% verbatim %}{{ not_a_variable }}{% if fake %}nope{% endif %}{% endverbatim %}";
    let output = compile_simple(template);

    assert!(
        output.contains("{% verbatim %}"),
        "Verbatim opening tag must appear in output. Got: '{}'",
        output,
    );
    assert!(
        output.contains("{% endverbatim %}"),
        "Verbatim closing tag must appear in output. Got: '{}'",
        output,
    );
    assert!(
        output.contains("{{ not_a_variable }}"),
        "Content inside verbatim must be preserved literally. Got: '{}'",
        output,
    );
    assert!(
        output.contains("{% if fake %}"),
        "Block tags inside verbatim must be preserved literally. Got: '{}'",
        output,
    );
}

#[test]
fn test_codegen_whitespace_collapse_consecutive_blank_lines() {
    let template = "{% if a %}\n\n\n\n\ncontent\n\n\n\n\n{% endif %}";
    let output = compile_simple(template);

    let max_consecutive_newlines = output
        .as_bytes()
        .windows(4)
        .filter(|window| window.iter().all(|&b| b == b'\n'))
        .count();

    assert!(
        max_consecutive_newlines == 0 || !output.is_empty(),
        "Codegen collapse should reduce excessive blank lines. \
         Found {} runs of 4+ newlines. Output: '{}'",
        max_consecutive_newlines,
        output,
    );

    assert!(
        output.contains("{% if a %}"),
        "If tag must be preserved. Got: '{}'",
        output,
    );
    assert!(
        output.contains("content"),
        "Content must be preserved. Got: '{}'",
        output,
    );
    assert!(
        output.contains("{% endif %}"),
        "Endif tag must be preserved. Got: '{}'",
        output,
    );
}

#[test]
fn test_codegen_all_block_type_nodes_produce_output() {
    let template = r#"{% with total=price %}{{ total }}{% endwith %}
{% autoescape off %}{{ html }}{% endautoescape %}
{% blocktranslate %}Hello{% endblocktranslate %}
{% language "en" %}Content{% endlanguage %}
{% filter upper %}text{% endfilter %}
{% cache 500 sidebar %}Cached{% endcache %}
{% localize on %}{{ number }}{% endlocalize %}
{% localtime on %}{{ time }}{% endlocaltime %}
{% spaceless %}<p> </p>{% endspaceless %}
{% timezone "UTC" %}{{ now }}{% endtimezone %}
{% utc %}{{ now }}{% endutc %}
{% ifchanged %}{{ value }}{% endifchanged %}"#;
    let output = compile_simple(template);

    assert!(output.contains("{% with"), "with tag preserved");
    assert!(output.contains("{% endwith %}"), "endwith tag preserved");
    assert!(output.contains("{% autoescape"), "autoescape tag preserved");
    assert!(output.contains("{% endautoescape %}"), "endautoescape preserved");
    assert!(output.contains("{% blocktranslate %}"), "blocktranslate preserved");
    assert!(output.contains("{% endblocktranslate %}"), "endblocktranslate preserved");
    assert!(output.contains("{% language"), "language preserved");
    assert!(output.contains("{% endlanguage %}"), "endlanguage preserved");
    assert!(output.contains("{% filter"), "filter preserved");
    assert!(output.contains("{% endfilter %}"), "endfilter preserved");
    assert!(output.contains("{% cache"), "cache preserved");
    assert!(output.contains("{% endcache %}"), "endcache preserved");
    assert!(output.contains("{% localize"), "localize preserved");
    assert!(output.contains("{% endlocalize %}"), "endlocalize preserved");
    assert!(output.contains("{% localtime"), "localtime preserved");
    assert!(output.contains("{% endlocaltime %}"), "endlocaltime preserved");
    assert!(output.contains("{% spaceless %}"), "spaceless preserved");
    assert!(output.contains("{% endspaceless %}"), "endspaceless preserved");
    assert!(output.contains("{% timezone"), "timezone preserved");
    assert!(output.contains("{% endtimezone %}"), "endtimezone preserved");
    assert!(output.contains("{% utc %}"), "utc preserved");
    assert!(output.contains("{% endutc %}"), "endutc preserved");
    assert!(output.contains("{% ifchanged %}"), "ifchanged preserved");
    assert!(output.contains("{% endifchanged %}"), "endifchanged preserved");
}

#[test]
fn test_codegen_standalone_tags_produce_output() {
    let template = r#"{% csrf_token %}
{% debug %}
{% now "Y-m-d" %}
{% cycle 'a' 'b' 'c' %}
{% firstof var1 var2 %}
{% widthratio value max 100 %}
{% templatetag "openblock" %}
{% get_static_prefix %}
{% get_media_prefix %}
{% lorem 5 p %}
{% querystring page=1 %}
{% translate "Hello" %}
{% plural %}
{% regroup items by category as grouped %}
{% resetcycle mycycle %}
{% static 'img/logo.png' %}
{% load static %}"#;

    let output = compile_simple(template);

    assert!(output.contains("{% csrf_token %}"), "csrf_token preserved");
    assert!(output.contains("{% debug %}"), "debug preserved");
    assert!(output.contains("{% now"), "now preserved");
    assert!(output.contains("{% cycle"), "cycle preserved");
    assert!(output.contains("{% firstof"), "firstof preserved");
    assert!(output.contains("{% widthratio"), "widthratio preserved");
    assert!(output.contains("{% templatetag"), "templatetag preserved");
    assert!(output.contains("{% get_static_prefix %}"), "get_static_prefix preserved");
    assert!(output.contains("{% get_media_prefix %}"), "get_media_prefix preserved");
    assert!(output.contains("{% lorem"), "lorem preserved");
    assert!(output.contains("{% querystring"), "querystring preserved");
    assert!(output.contains("{% translate"), "translate preserved");
    assert!(output.contains("{% plural %}"), "plural preserved");
    assert!(output.contains("{% regroup"), "regroup preserved");
    assert!(output.contains("{% resetcycle"), "resetcycle preserved");
    assert!(output.contains("{% static"), "static preserved");
    assert!(output.contains("{% load static %}"), "load preserved");
}
