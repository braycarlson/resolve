use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use resolve::compiler::Compiler;
use resolve::config::Config;
use resolve::reporter::{Reporter, Verbosity};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const DEFAULT_CONFIG_FILE: &str = "resolve.toml";

#[derive(Parser)]
#[command(name = "resolve")]
#[command(about = "Django template compiler")]
#[command(version)]
pub struct Cli {
    #[arg(short, long, help = "Path to resolve.toml config file")]
    pub config: Option<String>,

    #[arg(
        short,
        long,
        help = "Django project root (auto-detects templates and venv)"
    )]
    pub project: Option<PathBuf>,

    #[arg(short, long)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Compile templates
    Compile {
        #[arg(short, long, help = "Compile a single template")]
        template: Option<String>,

        #[arg(long, help = "Show what would be compiled without writing")]
        dry_run: bool,

        #[arg(long, help = "Disable incremental cache")]
        no_cache: bool,

        #[arg(long, help = "Skip vendor sync")]
        no_vendor: bool,
    },

    /// Validate all templates
    Validate,

    /// Manage vendor templates
    Vendor {
        #[command(subcommand)]
        vendor_cmd: VendorCommands,
    },

    /// Remove cache, vendor, and output directories
    Clean,
}

#[derive(Subcommand)]
pub enum VendorCommands {
    /// Sync vendor templates from installed packages
    Sync,

    /// Show vendor template status
    Status,
}

#[derive(Debug, Clone, Copy)]
enum ConfigSource {
    File,
    AutoDetected,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {}", error);
        std::process::exit(1);
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
    let (mut config, source) = match resolve_config(&cli, &reporter) {
        Ok(result) => result,
        Err(error) => {
            if matches!(cli.command, Commands::Clean) {
                return clean_fallback(&reporter);
            }

            return Err(error);
        }
    };

    if let Commands::Compile {
        no_cache,
        no_vendor,
        ..
    } = &cli.command
    {
        if *no_cache {
            config.incremental.enabled = false;
        }

        if *no_vendor {
            config.vendor.auto_detect = false;
        }
    }

    print_header(&reporter);
    print_config(&config, source, &reporter);

    let compiler = Compiler::new(&config, &reporter);

    match cli.command {
        Commands::Compile {
            template, dry_run, ..
        } => {
            if dry_run {
                compiler.dry_run()?;
            } else if let Some(path) = template {
                compiler.compile_single(&path)?;
            } else {
                compiler.compile_all()?;
            }
        }

        Commands::Validate => {
            compiler.validate()?;
        }

        Commands::Vendor { vendor_cmd } => match vendor_cmd {
            VendorCommands::Sync => {
                resolve::vendor::sync(&config, &reporter)?;
            }

            VendorCommands::Status => {
                resolve::vendor::status(&config, &reporter)?;
            }
        },

        Commands::Clean => {
            let directories = [
                config.cache_path().to_path_buf(),
                config.vendor_path().to_path_buf(),
                config.output_path().to_path_buf(),
            ];

            for directory in &directories {
                if directory.exists() {
                    std::fs::remove_dir_all(directory)?;
                    reporter.info(&format!("  Removed: {}", directory.display(),));
                }
            }
        }
    }

    Ok(())
}

fn resolve_config(cli: &Cli, reporter: &Reporter) -> Result<(Config, ConfigSource)> {
    if let Some(ref path) = cli.config {
        let config = resolve::config::load(path)?;
        return Ok((config, ConfigSource::File));
    }

    if PathBuf::from(DEFAULT_CONFIG_FILE).is_file() {
        let config = resolve::config::load(DEFAULT_CONFIG_FILE)?;
        return Ok((config, ConfigSource::File));
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

    let config =
        Config::from_project_directory(&project).map_err(|error| anyhow::anyhow!("{}", error))?;

    Ok((config, ConfigSource::AutoDetected))
}

fn print_header(reporter: &Reporter) {
    reporter.info(&format!(
        "{} v{}\n",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    ));
}

fn print_config(config: &Config, source: ConfigSource, reporter: &Reporter) {
    let source_label = match source {
        ConfigSource::File => "resolve.toml",
        ConfigSource::AutoDetected => "auto-detected",
    };

    reporter.info(&format!("Config ({})", source_label));

    reporter.info(&format!(
        "  {} template {}",
        config.paths.primary_templates.len(),
        if config.paths.primary_templates.len() == 1 {
            "directory"
        } else {
            "directories"
        },
    ));

    for directory in &config.paths.primary_templates {
        reporter.debug(&format!("    {}", directory.display()));
    }

    let venv_label = match config.vendor.virtual_environment_path {
        Some(ref path) => path.display().to_string(),
        None => "none".to_string(),
    };

    reporter.info(&format!("  venv: {}", venv_label));
    reporter.info(&format!("  output: {}", config.output_path().display()));

    let cache_label = if config.incremental.enabled {
        "incremental"
    } else {
        "disabled"
    };

    reporter.info(&format!(
        "  cache: {} ({})",
        config.cache_path().display(),
        cache_label,
    ));

    let vendor_label = if config.vendor.auto_detect {
        "auto-detect"
    } else {
        "disabled"
    };

    reporter.info(&format!(
        "  vendor: {} ({})\n",
        config.vendor_path().display(),
        vendor_label,
    ));
}

fn clean_fallback(reporter: &Reporter) -> Result<()> {
    print_header(reporter);

    let directories = [
        "resolve_templates",
        ".resolve_cache",
        ".resolve_vendor",
        ".cache",
        "vendor",
    ];

    for name in &directories {
        let path = PathBuf::from(name);

        if path.exists() {
            std::fs::remove_dir_all(&path)?;
            reporter.info(&format!("  Removed: {}", path.display()));
        }
    }

    Ok(())
}
