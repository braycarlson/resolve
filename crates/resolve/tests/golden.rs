mod common;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sha2::{Digest, Sha256};
use tempfile::TempDir;

use compiler::ast::{AstNode, Binding, Filter};
use compiler::lexer::{Lexer, Token};
use compiler::resolver::ResolveLimits;
use resolve::compiler::Compiler;
use resolve::config::Config;
use resolve::discovery::TemplateDiscovery;
use resolve::loader::{FsTemplateLoader, VendorIndex};
use resolve::reporter::Reporter;
use resolve::validator::Validator;

use common::create_test_config;

const BLESS: &str = "RESOLVE_GOLDEN=1 cargo test -p resolve --test golden";
const BINARY: &str = env!("CARGO_BIN_EXE_resolve");
const THROUGH_THE_CLI: &[&str] = &["broken", "inheritance"];

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixtures() -> PathBuf {
    workspace().join("fixtures")
}

fn goldens() -> PathBuf {
    workspace().join("crates/resolve/tests/golden")
}

fn blessing() -> bool {
    std::env::var_os("RESOLVE_GOLDEN").is_some()
}

fn names() -> Vec<String> {
    let mut found: Vec<String> = fs::read_dir(fixtures())
        .expect("the fixture directory is readable")
        .map(|entry| entry.expect("the entry is readable").path())
        .filter(|path| path.is_dir())
        .map(|path| {
            path.file_name()
                .expect("a directory has a name")
                .to_string_lossy()
                .into_owned()
        })
        .collect();

    found.sort();

    assert!(!found.is_empty(), "fixtures/ holds at least one project");

    found
}

fn golden(name: &str, actual: &str) {
    let path = goldens().join(name);

    if blessing() {
        let parent = path.parent().expect("a parent directory");

        fs::create_dir_all(parent).expect("the directory is created");
        fs::write(&path, actual).expect("the golden file is written");

        return;
    }

    let expected = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("{} is present; bless with {BLESS}", path.display()));

    assert_eq!(actual, expected, "{name} differs; bless with {BLESS}");
}

fn relative(path: &Path, base: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<String>>()
        .join("/")
}

fn joined(lines: &[String]) -> String {
    let mut text = lines.join("\n");

    if !text.is_empty() {
        text.push('\n');
    }

    text
}

fn files_under(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();

    if !directory.is_dir() {
        return found;
    }

    let mut stack = vec![directory.to_path_buf()];

    while let Some(current) = stack.pop() {
        let entries = fs::read_dir(&current).expect("the directory is readable");

        for entry in entries {
            let path = entry.expect("the entry is readable").path();

            if path.is_dir() {
                stack.push(path);
            } else {
                found.push(path);
            }
        }
    }

    found.sort();

    found
}

fn config_for(name: &str, output: &TempDir) -> Config {
    let fixture = fixtures().join(name);
    let declared = resolve::config::load(
        fixture
            .join("resolve.toml")
            .to_str()
            .expect("the fixture path is UTF-8"),
    )
    .expect("the fixture config loads");
    let mut config = create_test_config(output);

    config.paths.primary_templates = vec![fixture.join("templates")];
    config.vendor.vendor_directory = fixture.join("vendor");
    config.vendor.auto_detect = declared.vendor.auto_detect;
    config.entry_templates = declared.entry_templates;
    config.validation = declared.validation;
    config.incremental = declared.incremental;

    config
}

fn loader_limits(config: &Config) -> ResolveLimits {
    ResolveLimits {
        max_include_depth: config.validation.max_include_depth,
        max_inheritance_depth: config.validation.max_inheritance_depth,
    }
}

fn compiled(name: &str) -> (BTreeMap<String, String>, String) {
    let output = TempDir::new().expect("a temporary directory");
    let config = config_for(name, &output);
    let reporter = Reporter::silent();
    let compiler = Compiler::new(&config, &reporter);

    let outcome = compiler.compile_all();
    let root = config.output_path().to_path_buf();
    let mut emitted = BTreeMap::new();

    for path in files_under(&root) {
        let text = fs::read_to_string(&path).unwrap_or_else(|_| String::from("<not utf-8>\n"));

        emitted.insert(relative(&path, &root), text);
    }

    let report = match outcome {
        Ok(()) => String::from("ok\n"),
        Err(failure) => format!("error: {failure}\n"),
    };

    (emitted, report)
}

fn capture_output(name: &str) {
    let (emitted, report) = compiled(name);
    let mut listing: Vec<String> = emitted.keys().cloned().collect();

    listing.sort();

    golden(&format!("{name}/output.txt"), &joined(&listing));
    golden(&format!("{name}/compile.txt"), &report);

    for (path, text) in emitted {
        golden(&format!("{name}/output/{path}"), &text);
    }
}

fn capture_diagnostics(name: &str) {
    let output = TempDir::new().expect("a temporary directory");
    let config = config_for(name, &output);
    let discovery = TemplateDiscovery::new(&config);
    let index = discovery.scan().expect("the scan runs");
    let graph = discovery.dependencies(&index).expect("the graph builds");
    let entries = discovery.entries(&index, &graph);
    let vendor = VendorIndex::build(config.vendor_path());
    let mut validator = Validator::new();
    let fixture = fixtures().join(name);
    let root = fixture.display().to_string().replace('\\', "/");

    let text = match validator.validate(&index, &vendor, &entries, config.vendor_path()) {
        Ok(result) => {
            let mut lines = Vec::new();

            for error in &result.errors {
                lines.push(format!("error: {}", error.replace(&root, "<fixture>")));
            }

            for warning in &result.warnings {
                lines.push(format!("warning: {}", warning.replace(&root, "<fixture>")));
            }

            joined(&lines)
        }
        Err(failure) => format!(
            "aborted: {}\n",
            failure.to_string().replace(&root, "<fixture>")
        ),
    };

    golden(&format!("{name}/validate.txt"), &text);
}

fn templates_of(name: &str) -> Vec<(String, String)> {
    let root = fixtures().join(name).join("templates");

    files_under(&root)
        .iter()
        .map(|path| {
            (
                relative(path, &root),
                fs::read_to_string(path).expect("the template is readable"),
            )
        })
        .collect()
}

fn offset_of(source: &str, slice: &str) -> usize {
    (slice.as_ptr() as usize).saturating_sub(source.as_ptr() as usize)
}

fn token_row(source: &str, token: &Token<'_>) -> String {
    let (slice, kind) = match *token {
        Token::Text(text) => (text, String::from("text")),
        Token::Variable {
            raw, ref filters, ..
        } => {
            let names: Vec<&str> = filters.iter().map(|filter| filter.name).collect();
            let kind = if names.is_empty() {
                String::from("variable")
            } else {
                format!("variable:{}", names.join("|"))
            };

            (raw, kind)
        }
        Token::BlockStart { raw, tag, .. } => (raw, format!("block-start:{tag}")),
        Token::BlockEnd { raw, tag } => (raw, format!("block-end:{tag}")),
        Token::Comment(raw) => (raw, String::from("comment")),
        Token::Eof => ("", String::from("eof")),
    };

    if matches!(*token, Token::Eof) {
        return format!("{}:0 eof", source.len());
    }

    format!("{}:{} {kind}", offset_of(source, slice), slice.len())
}

fn capture_tokens(name: &str) {
    for (path, source) in templates_of(name) {
        let mut lexer = Lexer::new(&source);
        let text = match lexer.tokenize() {
            Ok(tokens) => {
                let rows: Vec<String> = tokens
                    .iter()
                    .map(|token| token_row(&source, token))
                    .collect();

                joined(&rows)
            }
            Err(failure) => format!("# error {failure}\n"),
        };

        golden(&format!("{name}/tokens/{path}.tokens"), &text);
    }
}

fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);

    out.push('"');

    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }

    out.push('"');

    out
}

fn field(name: &str, value: &str) -> String {
    format!(" {name}={}", quoted(value))
}

fn flag(name: &str, value: bool) -> String {
    format!(" {name}={value}")
}

fn optional(name: &str, value: Option<&String>) -> String {
    match value {
        Some(text) => field(name, text),
        None => format!(" {name}=none"),
    }
}

fn list(name: &str, values: &[String]) -> String {
    field(name, &values.join(","))
}

fn filters(values: &[Filter]) -> String {
    let rendered: Vec<String> = values
        .iter()
        .map(|filter| {
            if filter.arguments.is_empty() {
                filter.name.clone()
            } else {
                format!("{}:{}", filter.name, filter.arguments.join(","))
            }
        })
        .collect();

    list("filters", &rendered)
}

fn bindings(name: &str, values: &[Binding]) -> String {
    let rendered: Vec<String> = values
        .iter()
        .map(|binding| format!("{}={}", binding.name, binding.value))
        .collect();

    list(name, &rendered)
}

fn print_node(node: &AstNode, depth: usize, out: &mut Vec<String>) {
    let padding = "  ".repeat(depth);
    let mut children: Vec<(&str, &[AstNode])> = Vec::new();
    let head = match node {
        AstNode::Text(text) => format!("Text{}", field("content", &text.content)),
        AstNode::Variable(variable) => format!(
            "Variable{}{}{}",
            field("raw", &variable.raw),
            field("expression", &variable.expression),
            filters(&variable.filters)
        ),
        AstNode::Block(block) => {
            children.push(("content", &block.content));

            format!(
                "Block{}{}{}",
                field("raw", &block.raw),
                field("name", &block.name),
                flag("has_super_reference", block.has_super_reference)
            )
        }
        AstNode::Extends(extends) => format!(
            "Extends{}{}{}",
            field("raw", &extends.raw),
            field("parent_path", &extends.parent_path),
            flag("is_literal", extends.is_literal)
        ),
        AstNode::Include(include) => format!(
            "Include{}{}{}{}{}",
            field("raw", &include.raw),
            field("path", &include.path),
            bindings("with_variables", &include.with_variables),
            flag("only", include.only),
            flag("is_literal", include.is_literal)
        ),
        AstNode::If(node) => {
            children.push(("true_branch", &node.true_branch));

            for elif in &node.elif_branches {
                children.push(("elif_branch", &elif.body));
            }

            if let Some(branch) = &node.else_branch {
                children.push(("else_branch", branch));
            }

            let conditions: Vec<String> = node
                .elif_branches
                .iter()
                .map(|elif| elif.condition.clone())
                .collect();

            format!(
                "If{}{}{}",
                field("raw", &node.raw),
                field("condition", &node.condition),
                list("elif_conditions", &conditions)
            )
        }
        AstNode::For(node) => {
            children.push(("body", &node.body));

            if let Some(branch) = &node.empty_branch {
                children.push(("empty_branch", branch));
            }

            format!(
                "For{}{}{}",
                field("raw", &node.raw),
                field("variable", &node.variable),
                field("iterable", &node.iterable)
            )
        }
        AstNode::With(node) => {
            children.push(("body", &node.body));

            format!(
                "With{}{}",
                field("raw", &node.raw),
                bindings("bindings", &node.bindings)
            )
        }
        AstNode::Load(node) => format!(
            "Load{}{}",
            field("raw", &node.raw),
            list("libraries", &node.libraries)
        ),
        AstNode::Static(node) => {
            format!("Static{}{}", field("raw", &node.raw), field("path", &node.path))
        }
        AstNode::Csrftoken(node) => format!("Csrftoken{}", field("raw", &node.raw)),
        AstNode::CommentBlock(node) => format!(
            "CommentBlock{}{}",
            field("raw", &node.raw),
            field("content", &node.content)
        ),
        AstNode::Autoescape(node) => {
            children.push(("body", &node.body));

            format!(
                "Autoescape{}{}",
                field("raw", &node.raw),
                field("value", &node.value)
            )
        }
        AstNode::Blocktranslate(node) => {
            children.push(("body", &node.body));

            format!(
                "Blocktranslate{}{}",
                field("raw", &node.raw),
                list("modifiers", &node.modifiers)
            )
        }
        AstNode::Trans(node) => format!(
            "Trans{}{}",
            field("raw", &node.raw),
            field("message", &node.message)
        ),
        AstNode::Language(node) => {
            children.push(("body", &node.body));

            format!(
                "Language{}{}",
                field("raw", &node.raw),
                field("language", &node.language)
            )
        }
        AstNode::Verbatim(node) => format!(
            "Verbatim{}{}",
            field("raw", &node.raw),
            field("content", &node.content)
        ),
        AstNode::TemplateTag(node) => format!(
            "TemplateTag{}{}",
            field("raw", &node.raw),
            field("format", &node.format)
        ),
        AstNode::Ifchanged(node) => {
            children.push(("true_branch", &node.true_branch));

            if let Some(branch) = &node.else_branch {
                children.push(("else_branch", branch));
            }

            format!(
                "Ifchanged{}{}",
                field("raw", &node.raw),
                optional("condition", node.condition.as_ref())
            )
        }
        AstNode::FilterBlock(node) => {
            children.push(("body", &node.body));

            format!(
                "FilterBlock{}{}",
                field("raw", &node.raw),
                field("filter", &node.filter)
            )
        }
        AstNode::Now(node) => format!(
            "Now{}{}",
            field("raw", &node.raw),
            field("format", &node.format)
        ),
        AstNode::Regroup(node) => format!(
            "Regroup{}{}{}{}",
            field("raw", &node.raw),
            field("list", &node.list),
            field("field", &node.field),
            field("as_variable", &node.as_variable)
        ),
        AstNode::Cycle(node) => format!(
            "Cycle{}{}{}{}",
            field("raw", &node.raw),
            list("values", &node.values),
            optional("as_variable", node.as_variable.as_ref()),
            flag("output", node.output)
        ),
        AstNode::Firstof(node) => format!(
            "Firstof{}{}",
            field("raw", &node.raw),
            list("variables", &node.variables)
        ),
        AstNode::Widthratio(node) => format!(
            "Widthratio{}{}{}{}",
            field("raw", &node.raw),
            field("value", &node.value),
            field("maximum", &node.maximum),
            field("divisor", &node.divisor)
        ),
        AstNode::JsonScript(node) => format!(
            "JsonScript{}{}{}",
            field("raw", &node.raw),
            field("variable", &node.variable),
            field("id", &node.id)
        ),
        AstNode::CaptureAs(node) => format!(
            "CaptureAs{}{}{}{}",
            field("raw", &node.raw),
            field("tag", &node.tag),
            field("content", &node.content),
            field("variable_name", &node.variable_name)
        ),
        AstNode::Cache(node) => {
            children.push(("body", &node.body));

            format!(
                "Cache{}{}{}",
                field("raw", &node.raw),
                field("timeout", &node.timeout),
                field("name", &node.name)
            )
        }
        AstNode::Localize(node) => {
            children.push(("body", &node.body));

            format!(
                "Localize{}{}",
                field("raw", &node.raw),
                field("value", &node.value)
            )
        }
        AstNode::Localtime(node) => {
            children.push(("body", &node.body));

            format!(
                "Localtime{}{}",
                field("raw", &node.raw),
                field("value", &node.value)
            )
        }
        AstNode::Spaceless(node) => {
            children.push(("body", &node.body));

            format!("Spaceless{}", field("raw", &node.raw))
        }
        AstNode::Timezone(node) => {
            children.push(("body", &node.body));

            format!(
                "Timezone{}{}",
                field("raw", &node.raw),
                field("timezone", &node.timezone)
            )
        }
        AstNode::Utc(node) => {
            children.push(("body", &node.body));

            format!("Utc{}", field("raw", &node.raw))
        }
        AstNode::Debug(node) => format!("Debug{}", field("raw", &node.raw)),
        AstNode::GetStaticPrefix(node) => format!(
            "GetStaticPrefix{}{}",
            field("raw", &node.raw),
            optional("variable_name", node.variable_name.as_ref())
        ),
        AstNode::GetMediaPrefix(node) => format!(
            "GetMediaPrefix{}{}",
            field("raw", &node.raw),
            optional("variable_name", node.variable_name.as_ref())
        ),
        AstNode::Resetcycle(node) => format!(
            "Resetcycle{}{}",
            field("raw", &node.raw),
            field("name", &node.name)
        ),
        AstNode::Lorem(node) => format!(
            "Lorem{}{}{}{}",
            field("raw", &node.raw),
            field("count", &node.count),
            field("method", &node.method),
            flag("random", node.random)
        ),
        AstNode::Querystring(node) => format!(
            "Querystring{}{}{}",
            field("raw", &node.raw),
            bindings("parameters", &node.parameters),
            optional("variable_name", node.variable_name.as_ref())
        ),
        AstNode::Translate(node) => format!(
            "Translate{}{}{}{}",
            field("raw", &node.raw),
            field("message", &node.message),
            optional("variable_name", node.variable_name.as_ref()),
            flag("noop", node.noop)
        ),
        AstNode::Plural(node) => format!(
            "Plural{}{}",
            field("raw", &node.raw),
            field("content", &node.content)
        ),
    };

    out.push(format!("{padding}({head})"));

    for (label, nodes) in children {
        out.push(format!("{padding}  :{label}"));

        for child in nodes {
            print_node(child, depth + 2, out);
        }
    }
}

fn capture_ast(name: &str) {
    for (path, source) in templates_of(name) {
        let text = match compiler::parser::parse(&source) {
            Ok(output) => {
                let mut lines = Vec::new();

                for node in &output.nodes {
                    print_node(node, 0, &mut lines);
                }

                joined(&lines)
            }
            Err(failure) => format!("# error {failure}\n"),
        };

        golden(&format!("{name}/ast/{path}.ast"), &text);
    }
}

fn capture_errors(name: &str) {
    let output = TempDir::new().expect("a temporary directory");
    let config = config_for(name, &output);
    let discovery = TemplateDiscovery::new(&config);
    let index = discovery.scan().expect("the scan runs");
    let vendor = VendorIndex::build(config.vendor_path());
    let loader = FsTemplateLoader::new(&index, &vendor);
    let limits = loader_limits(&config);
    let root = fixtures().join(name).join("templates");
    let mut lines = Vec::new();

    for (path, source) in templates_of(name) {
        let parsed = match compiler::parser::parse(&source) {
            Ok(output) => output.nodes,
            Err(failure) => {
                lines.push(format!("{path}: parse: {failure}"));

                continue;
            }
        };

        let full = root.join(&path);
        let inherited =
            match compiler::resolver::inheritance::resolve(
                parsed,
                &path,
                Some(full),
                &loader,
                &limits,
            ) {
                Ok(nodes) => nodes,
                Err(failure) => {
                    lines.push(format!("{path}: inheritance: {failure}"));

                    continue;
                }
            };

        match compiler::resolver::inclusion::resolve(inherited, &path, &loader, &limits) {
            Ok(_) => lines.push(format!("{path}: ok")),
            Err(failure) => lines.push(format!("{path}: inclusion: {failure}")),
        }
    }

    golden(&format!("{name}/errors.txt"), &joined(&lines));
}

fn staged(name: &str) -> TempDir {
    let source = fixtures().join(name);
    let target = TempDir::new().expect("a temporary directory");

    for path in files_under(&source) {
        let destination = target.path().join(relative(&path, &source));
        let parent = destination.parent().expect("a parent directory");

        fs::create_dir_all(parent).expect("the staging directory is created");
        fs::copy(&path, &destination).expect("the fixture is staged");
    }

    target
}

fn normalized(text: &str, root: &Path) -> String {
    let staging = root.display().to_string().replace('\\', "/");
    let mut out = text
        .replace('\\', "/")
        .replace(&staging, "<root>")
        .replace(env!("CARGO_PKG_VERSION"), "<version>");

    while let Some(start) = out.find("Done in ") {
        let tail_start = start + "Done in ".len();
        let tail = out.get(tail_start..).unwrap_or_default();

        let Some(end) = tail.find('s') else {
            break;
        };

        out.replace_range(tail_start..tail_start + end, "<duration>");

        let resumed = tail_start + "<duration>".len() + 1;

        if out.get(resumed..).is_none_or(|rest| !rest.contains("Done in ")) {
            break;
        }
    }

    out
}

fn report_of(output: &Output, root: &Path) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    normalized(
        &format!(
            "# exit {}\n--- stdout ---\n{stdout}--- stderr ---\n{stderr}",
            output.status.code().unwrap_or(-1)
        ),
        root,
    )
}

fn invoked(root: &Path, arguments: &[&str]) -> Output {
    Command::new(BINARY)
        .args(["--config", "resolve.toml"])
        .args(arguments)
        .current_dir(root)
        .env_remove("CLICOLOR_FORCE")
        .env_remove("FORCE_COLOR")
        .env("NO_COLOR", "1")
        .env("TERM", "dumb")
        .env("LC_ALL", "C.UTF-8")
        .output()
        .expect("the binary runs")
}

fn capture_cli(name: &str) {
    let staging = staged(name);
    let root = staging.path();

    golden(
        &format!("{name}/cli-compile.txt"),
        &report_of(&invoked(root, &["compile"]), root),
    );
    golden(
        &format!("{name}/cli-validate.txt"),
        &report_of(&invoked(root, &["validate"]), root),
    );
}

fn capture_incremental(name: &str, leaf: &str) {
    let staging = staged(name);
    let root = staging.path();
    let config = root.join("resolve.toml");
    let declared = fs::read_to_string(&config).expect("the staged config is readable");

    fs::write(
        &config,
        declared
            .replace("enabled = false", "enabled = true")
            .replace("track_file_hashes = false", "track_file_hashes = true"),
    )
    .expect("the staged config is rewritten");

    invoked(root, &["compile"]);

    golden(
        &format!("{name}/cli-incremental.txt"),
        &report_of(&invoked(root, &["compile"]), root),
    );

    let template = root.join("templates").join(leaf);
    let source = fs::read_to_string(&template).expect("the leaf template is readable");

    fs::write(&template, format!("{source}{{# touched #}}\n"))
        .expect("the leaf template is rewritten");

    golden(
        &format!("{name}/cli-touched.txt"),
        &report_of(&invoked(root, &["compile"]), root),
    );
}

fn manifest() -> String {
    let root = goldens();
    let mut lines = Vec::new();

    for path in files_under(&root) {
        let name = relative(&path, &root);

        if name == "MANIFEST.txt" {
            continue;
        }

        let bytes = fs::read(&path).expect("the golden file is readable");
        let digest = Sha256::digest(&bytes);
        let mut hash = String::with_capacity(64);

        for byte in digest.iter() {
            write!(hash, "{byte:02x}").expect("hex formatting does not fail");
        }

        lines.push(format!("{name}  {hash}"));
    }

    lines.sort();

    joined(&lines)
}

#[test]
fn the_fixture_goldens_match() {
    for name in names() {
        capture_output(&name);
        capture_diagnostics(&name);
        capture_tokens(&name);
        capture_ast(&name);
        capture_errors(&name);

        if THROUGH_THE_CLI.contains(&name.as_str()) {
            capture_cli(&name);
        }
    }

    capture_incremental("inheritance", "page.html");

    golden("MANIFEST.txt", &manifest());
}
