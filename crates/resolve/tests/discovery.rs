mod common;

use tempfile::TempDir;

use resolve::discovery::TemplateDiscovery;
use resolve::loader::{MANIFEST_FILE, VendorIndex};
use resolve::validator::Validator;

use common::{create_test_config, write_template};

#[test]
fn test_validator_finds_an_include_whatever_its_extension() {
    let temp_dir = TempDir::new().unwrap();

    write_template(
        &temp_dir,
        "page.html",
        concat!(
            "{% include 'icon/mark.svg' %}",
            "{% include 'email/subject.txt' %}",
            "{% include 'icon/absent.svg' %}",
        ),
    );

    write_template(&temp_dir, "icon/mark.svg", "<svg></svg>");
    write_template(&temp_dir, "email/subject.txt", "Welcome");

    let config = create_test_config(&temp_dir);
    let discovery = TemplateDiscovery::new(&config);
    let index = discovery.scan().unwrap();
    let vendor = VendorIndex::build(config.vendor_path());
    let entries = vec![String::from("page.html")];
    let mut validator = Validator::new();

    let result = validator
        .validate(&index, &vendor, &entries, config.vendor_path())
        .unwrap();

    assert_eq!(
        result.errors,
        vec![String::from(
            "[ERROR] page.html: Include template not found: icon/absent.svg",
        )],
        "Django loads a template of any extension, so only the include with no file behind it \
         is missing. Got: {:?}",
        result.errors,
    );
}

#[test]
fn test_only_an_html_page_nothing_references_is_an_entry() {
    let temp_dir = TempDir::new().unwrap();

    write_template(&temp_dir, "page.html", "{% include 'icon/mark.svg' %}");
    write_template(&temp_dir, "icon/mark.svg", "<svg></svg>");
    write_template(&temp_dir, "robots.txt", "User-agent: *");

    let mut config = create_test_config(&temp_dir);

    config.entry_templates.auto_discover = true;

    let discovery = TemplateDiscovery::new(&config);
    let index = discovery.scan().unwrap();
    let graph = discovery.dependencies(&index).unwrap();

    assert_eq!(
        discovery.entries(&index, &graph),
        vec![String::from("page.html")],
        "A file that is not HTML resolves by name but is never compiled or validated as a root",
    );
}

#[test]
fn test_the_vendor_manifest_is_not_a_template() {
    let temp_dir = TempDir::new().unwrap();

    write_template(&temp_dir, "page.html", "Page");
    write_template(&temp_dir, &format!("vendor/{MANIFEST_FILE}"), "ninja");
    write_template(&temp_dir, "vendor/registration/subject.txt", "Reset");

    let config = create_test_config(&temp_dir);
    let index = TemplateDiscovery::new(&config).scan().unwrap();
    let vendor = VendorIndex::build(config.vendor_path());

    assert!(
        vendor.find("registration/subject.txt").is_some(),
        "A vendor template that is not HTML must resolve",
    );

    assert!(
        vendor.find(MANIFEST_FILE).is_none(),
        "The vendor manifest is bookkeeping, not a template",
    );

    assert!(
        index
            .templates
            .keys()
            .all(|name| !name.ends_with(MANIFEST_FILE)),
        "The vendor manifest must not be indexed. Got: {:?}",
        index.templates.keys().collect::<Vec<_>>(),
    );
}
