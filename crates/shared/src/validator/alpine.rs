use rustc_hash::FxHashSet;

const CORE_DIRECTIVES: &[&str] = &[
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

const PLUGIN_DIRECTIVES: &[&str] = &[
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

const MAGIC_PROPERTIES: &[&str] = &[
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

const EVENT_MODIFIERS: &[&str] = &[
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

const KEYBOARD_MODIFIERS: &[&str] = &[
    ".shift", ".ctrl", ".alt", ".meta", ".enter", ".escape", ".tab", ".space", ".up", ".down",
    ".left", ".right",
];

const MODEL_MODIFIERS: &[&str] = &[".lazy", ".number", ".debounce", ".throttle", ".fill"];

const TRANSITION_MODIFIERS: &[&str] = &[".duration", ".delay", ".opacity", ".scale"];

const TRANSITION_CLASSES: &[&str] = &[
    "x-transition:enter",
    "x-transition:enter-start",
    "x-transition:enter-end",
    "x-transition:leave",
    "x-transition:leave-start",
    "x-transition:leave-end",
];

const DIRECTIVE_PREFIX_ALLOWLIST: &[&str] = &[
    "x-bind:",
    "x-on:",
    "x-transition",
    "x-sort",
    "x-intersect",
    "x-collapse",
    "x-anchor",
];

const ALL_MODIFIER_GROUPS: &[&[&str]] = &[
    EVENT_MODIFIERS,
    KEYBOARD_MODIFIERS,
    MODEL_MODIFIERS,
    TRANSITION_MODIFIERS,
];

const MODIFIER_PARTS_MAX: u32 = 1_000;

pub fn all() -> FxHashSet<String> {
    let mut directives = FxHashSet::default();

    for &directive in CORE_DIRECTIVES {
        directives.insert(directive.to_string());
    }

    for &directive in PLUGIN_DIRECTIVES {
        directives.insert(directive.to_string());
    }

    for &directive in TRANSITION_CLASSES {
        directives.insert(directive.to_string());
    }

    directives
}

pub fn magic() -> FxHashSet<String> {
    MAGIC_PROPERTIES
        .iter()
        .map(|string| string.to_string())
        .collect()
}

pub fn events() -> FxHashSet<String> {
    EVENT_MODIFIERS
        .iter()
        .map(|string| string.to_string())
        .collect()
}

pub fn keyboard() -> FxHashSet<String> {
    KEYBOARD_MODIFIERS
        .iter()
        .map(|string| string.to_string())
        .collect()
}

pub fn model() -> FxHashSet<String> {
    MODEL_MODIFIERS
        .iter()
        .map(|string| string.to_string())
        .collect()
}

pub fn transitions() -> FxHashSet<String> {
    TRANSITION_MODIFIERS
        .iter()
        .map(|string| string.to_string())
        .collect()
}

pub fn validate_directive(directive: &str) -> Result<(), String> {
    assert!(!directive.is_empty(), "directive must not be empty",);

    let directives = all();

    if directives.contains(directive) {
        return Ok(());
    }

    if directive.starts_with(':') || directive.starts_with('@') {
        return Ok(());
    }

    for prefix in DIRECTIVE_PREFIX_ALLOWLIST {
        if directive.starts_with(prefix) {
            return Ok(());
        }
    }

    Err(format!("Unknown Alpine.js directive: '{}'", directive))
}

pub fn validate_magic_property(property: &str) -> Result<(), String> {
    assert!(!property.is_empty(), "property must not be empty",);

    let properties = magic();

    if properties.contains(property) {
        Ok(())
    } else {
        Err(format!("Unknown Alpine.js magic property: '{}'", property))
    }
}

pub fn validate_modifiers(content: &str) -> Vec<String> {
    assert!(!content.is_empty(), "content must not be empty",);

    let mut errors = Vec::new();

    let mut modifiers = FxHashSet::default();

    for group in ALL_MODIFIER_GROUPS {
        for &modifier in *group {
            modifiers.insert(modifier.to_string());
        }
    }

    let mut iterations: u32 = 0;

    for part in content.split('.') {
        iterations += 1;

        assert!(
            iterations <= MODIFIER_PARTS_MAX,
            "validate_modifiers exceeded {} iterations",
            MODIFIER_PARTS_MAX,
        );

        if part.is_empty() {
            continue;
        }

        let modifier = format!(".{}", part);

        if modifiers.contains(&modifier) {
            continue;
        }

        if modifier.starts_with(".debounce") || modifier.starts_with(".throttle") {
            continue;
        }

        errors.push(format!("Unknown modifier: '{}'", modifier));
    }

    errors
}
