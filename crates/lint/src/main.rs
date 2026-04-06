use std::path::PathBuf;
use std::process;

use anyhow::Result;
use clap::Parser;

use shared::config::Config;
use shared::discovery::TemplateDiscovery;
use shared::loader::VendorIndex;
use shared::reporter::{Reporter, Verbosity};
use shared::validator::Validator;

const DEFAULT_CONFIG_FILE: &str = "resolve.toml";

#[derive(Parser)]
#[command(name = "lint")]
#[command(about = "Lint Django templates for broken extends and includes")]
#[command(version)]
struct Cli {
    #[arg(short, long, help = "Path to resolve.toml config file")]
    config: Option<String>,

    #[arg(
        short,
        long,
        help = "Django project root (auto-detects templates and venv)"
    )]
    project: Option<PathBuf>,

    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {}", error);
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    let verbosity = if cli.verbose {
        Verbosity::Verbose
    } else {
        Verbosity::Normal
    };

    let reporter = Reporter::console(verbosity);
    let config = resolve_config(&cli, &reporter)?;

    sync_vendor(&config, &reporter)?;

    let discovery = TemplateDiscovery::new(&config);
    let index = discovery.scan()?;
    let graph = discovery.dependencies(&index)?;
    let entries = discovery.entries(&index, &graph);
    let vendor = VendorIndex::build(config.vendor_path());

    reporter.info(&format!(
        "Validating {} entry template{}...",
        entries.len(),
        if entries.len() == 1 { "" } else { "s" },
    ));

    let mut validator = Validator::new();
    let result = validator.validate(&index, &vendor, &entries, config.vendor_path())?;

    for error in &result.errors {
        reporter.error(error);
    }

    for warning in &result.warnings {
        reporter.warn(warning);
    }

    if !result.errors.is_empty() {
        reporter.info(&format!(
            "\nValidation failed: {} error{}",
            result.errors.len(),
            if result.errors.len() == 1 { "" } else { "s" },
        ));

        process::exit(1);
    }

    reporter.info("Validation passed.");

    Ok(())
}

fn sync_vendor(config: &Config, reporter: &Reporter) -> Result<()> {
    let manager = resolve::vendor::VendorManager::new(config, reporter);

    if !manager.vendor_exists() || manager.is_vendor_stale()? {
        let count = manager.sync()?;

        reporter.info(&format!(
            "Vendor: {} template{} synced",
            count,
            if count == 1 { "" } else { "s" },
        ));
    } else {
        reporter.debug("Vendor templates are up to date");
    }

    Ok(())
}

fn resolve_config(cli: &Cli, reporter: &Reporter) -> Result<Config> {
    if let Some(ref path) = cli.config {
        return shared::config::load(path);
    }

    if PathBuf::from(DEFAULT_CONFIG_FILE).is_file() {
        return shared::config::load(DEFAULT_CONFIG_FILE);
    }

    let project = match cli.project {
        Some(ref path) => path.clone(),
        None => std::env::current_dir()?,
    };

    assert!(
        project.is_dir(),
        "project directory must exist: {:?}",
        project,
    );

    reporter.debug(&format!("Auto-detecting from: {}", project.display(),));

    Config::from_project_directory(&project)
        .map_err(|error| anyhow::anyhow!("{}", error))
}
