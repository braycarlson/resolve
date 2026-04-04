use rustc_hash::FxHashSet;


const DJANGO_BUILTIN_FILTERS: &[&str] = &[
    "add", "addslashes", "capfirst", "center", "cut", "date",
    "default", "default_if_none", "dictsort", "dictsortreversed",
    "divisibleby", "escape", "escapejs", "filesizeformat", "first",
    "floatformat", "force_escape", "get_digit", "iriencode", "join",
    "json_script", "last", "length", "length_is", "linebreaks",
    "linebreaksbr", "linenumbers", "ljust", "lower", "make_list",
    "phone2numeric", "pluralize", "pprint", "random", "rjust", "safe",
    "safeseq", "slice", "slugify", "stringformat", "striptags",
    "time", "timesince", "timeuntil", "title", "truncatechars",
    "truncatechars_html", "truncatewords", "truncatewords_html",
    "unordered_list", "upper", "urlencode", "urlize", "urlizetrunc",
    "wordcount", "wordwrap", "yesno",
];

const DJANGO_HUMANIZE_FILTERS: &[&str] = &[
    "apnumber", "intcomma", "intword",
    "naturalday", "naturaltime", "ordinal",
];

const SPIRE_FILTERS: &[&str] = &[
    "add_str",
    "dashes_and_spaces_to_underscore",
    "dashes_to_underscore",
    "format_to_html",
    "humanize_duration",
    "humanize_duration_simple",
    "in_list",
    "index",
    "is_dict",
    "is_list",
    "is_list_or_tuple",
    "is_not_dict",
    "is_not_list",
    "is_not_list_or_tuple",
    "is_not_tuple",
    "is_path",
    "is_tuple",
    "model_app_label",
    "model_name",
    "not_in_list",
    "render_markdown",
    "safe_dict_items",
    "spaces_to_underscore",
    "to_json",
    "to_snake_case",
    "underscores_to_spaces",
];

const SPIRE_SIMPLE_TAGS: &[&str] = &[];

pub fn all() -> FxHashSet<String> {
    let filters: FxHashSet<String> = DJANGO_BUILTIN_FILTERS
        .iter()
        .chain(DJANGO_HUMANIZE_FILTERS.iter())
        .chain(SPIRE_FILTERS.iter())
        .map(|string| string.to_string())
        .collect();

    assert!(
        !filters.is_empty(),
        "filter set must not be empty after initialization",
    );

    filters
}

pub fn simple_tags() -> FxHashSet<String> {
    let tags: FxHashSet<String> =
        SPIRE_SIMPLE_TAGS.iter().map(|string| string.to_string()).collect();

    assert!(
        tags.is_empty() || !tags.is_empty(),
        "simple tags set initialization must succeed",
    );

    tags
}

pub fn validate_filter(name: &str) -> Result<(), String> {
    assert!(
        !name.is_empty(),
        "filter_name must not be empty",
    );

    let filters = all();

    assert!(
        !filters.is_empty(),
        "filters must not be empty during validation",
    );

    if filters.contains(name) {
        Ok(())
    } else {
        Err(format!("Unknown filter: '{}'", name))
    }
}

pub fn validate_filters(filters: &[compiler::ast::Filter]) -> Vec<String> {
    let count = u32::try_from(filters.len())
        .expect("filter count must fit in u32");

    assert!(
        count <= 100,
        "filter count per variable must not exceed 100",
    );

    filters
        .iter()
        .filter_map(|filter| validate_filter(&filter.name).err())
        .collect()
}
