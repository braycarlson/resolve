use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use walkdir::WalkDir;


const DEFAULT_OUTPUT_DIRECTORY: &str = "resolve_templates";
const DEFAULT_CACHE_DIRECTORY: &str = ".resolve_cache";
const DEFAULT_VENDOR_DIRECTORY: &str = ".resolve_vendor";

const VENV_PROBE_NAMES: &[&str] = &[".venv", "venv"];

const PROJECT_WALK_ENTRIES_MAX: u32 = 100_000;

const EXCLUDED_DIRECTORY_SEGMENTS: &[&str] = &[
    ".venv",
    ".git",
    ".resolve_cache",
    ".resolve_vendor",
    "__pycache__",
    "resolve_templates",
    "node_modules",
    "static",
    "venv",
];

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("configuration field '{0}' must not be empty")]
    EmptyField(&'static str),

    #[error("configuration field '{0}' must contain at least one entry")]
    EmptyList(&'static str),

    #[error(
        "configuration field '{field}' value {value} is out of range ({min}..={max})"
    )]
    OutOfRange {
        field: &'static str,
        value: u32,
        min: u32,
        max: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub compiler: CompilerConfig,
    pub paths: PathsConfig,

    #[serde(default)]
    pub vendor: VendorConfig,

    #[serde(default)]
    pub entry_templates: EntryTemplatesConfig,

    #[serde(default)]
    pub validation: ValidationConfig,

    #[serde(default)]
    pub incremental: IncrementalConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerConfig {
    pub output_directory: PathBuf,
    pub cache_directory: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub primary_templates: Vec<PathBuf>,

    #[serde(default)]
    pub app_templates: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VendorConfig {
    #[serde(default)]
    pub auto_detect: bool,

    #[serde(default)]
    pub vendor_directory: PathBuf,

    #[serde(default)]
    pub virtual_environment_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntryTemplatesConfig {
    #[serde(default)]
    pub auto_discover: bool,

    #[serde(default)]
    pub explicit: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    #[serde(default)]
    pub strict: bool,

    #[serde(default)]
    pub warn_undefined_vars: bool,

    #[serde(default = "default_max_include_depth")]
    pub max_include_depth: u32,

    #[serde(default = "default_max_inheritance_depth")]
    pub max_inheritance_depth: u32,

    #[serde(default)]
    pub skip_url_validation: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            strict: false,
            warn_undefined_vars: false,
            max_include_depth: default_max_include_depth(),
            max_inheritance_depth: default_max_inheritance_depth(),
            skip_url_validation: false,
        }
    }
}

fn default_max_include_depth() -> u32 {
    20
}

fn default_max_inheritance_depth() -> u32 {
    10
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IncrementalConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub track_file_hashes: bool,
}

pub fn load(path: &str) -> Result<Config> {
    assert!(
        !path.is_empty(),
        "config_path must not be empty",
    );

    let content = fs::read_to_string(path)?;

    assert!(
        !content.is_empty(),
        "config file must not be empty: {}",
        path,
    );

    let config: Config = toml::from_str(&content)?;
    config.validate()?;

    Ok(config)
}

impl Config {
    pub fn from_params(
        directories: Vec<PathBuf>,
        output: PathBuf,
        cache: PathBuf,
        vendor: PathBuf,
        environment: Option<PathBuf>,
    ) -> Result<Self, ConfigError> {
        assert!(
            !directories.is_empty(),
            "template_directories must not be empty",
        );

        assert!(
            !output.as_os_str().is_empty(),
            "output_directory must not be empty",
        );

        let config = Self {
            compiler: CompilerConfig {
                output_directory: output,
                cache_directory: cache,
            },
            paths: PathsConfig {
                primary_templates: directories,
                app_templates: Vec::new(),
            },
            vendor: VendorConfig {
                auto_detect: true,
                vendor_directory: vendor,
                virtual_environment_path: environment,
            },
            entry_templates: EntryTemplatesConfig {
                auto_discover: true,
                explicit: Vec::new(),
            },
            validation: ValidationConfig::default(),
            incremental: IncrementalConfig {
                enabled: true,
                track_file_hashes: true,
            },
        };

        config.validate()?;

        Ok(config)
    }

    pub fn from_project_directory(
        project: &Path,
    ) -> Result<Self, ConfigError> {
        assert!(
            project.is_dir(),
            "project_directory must be a directory: {:?}",
            project,
        );

        let directories = discover_templates(project);

        if directories.is_empty() {
            return Err(ConfigError::EmptyList("template_directories"));
        }

        let environment = discover_environment(project);

        let output = project.join(DEFAULT_OUTPUT_DIRECTORY);
        let cache = project.join(DEFAULT_CACHE_DIRECTORY);
        let vendor = project.join(DEFAULT_VENDOR_DIRECTORY);

        Self::from_params(
            directories,
            output,
            cache,
            vendor,
            environment,
        )
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.compiler.output_directory.as_os_str().is_empty() {
            return Err(ConfigError::EmptyField("compiler.output_directory"));
        }

        if self.compiler.cache_directory.as_os_str().is_empty() {
            return Err(ConfigError::EmptyField("compiler.cache_directory"));
        }

        if self.paths.primary_templates.is_empty() {
            return Err(ConfigError::EmptyList("paths.primary_templates"));
        }

        if self.validation.max_include_depth == 0
            || self.validation.max_include_depth > 128
        {
            return Err(ConfigError::OutOfRange {
                field: "validation.max_include_depth",
                value: self.validation.max_include_depth,
                min: 1,
                max: 128,
            });
        }

        if self.validation.max_inheritance_depth == 0
            || self.validation.max_inheritance_depth > 128
        {
            return Err(ConfigError::OutOfRange {
                field: "validation.max_inheritance_depth",
                value: self.validation.max_inheritance_depth,
                min: 1,
                max: 128,
            });
        }

        assert!(
            !self.compiler.output_directory.as_os_str().is_empty(),
            "compiler.output_directory must not be empty after validation",
        );

        assert!(
            !self.compiler.cache_directory.as_os_str().is_empty(),
            "compiler.cache_directory must not be empty after validation",
        );

        assert!(
            !self.paths.primary_templates.is_empty(),
            "paths.primary_templates must not be empty after validation",
        );

        assert!(
            self.validation.max_include_depth > 0
                && self.validation.max_include_depth <= 128,
            "max_include_depth must be in range 1..=128 after validation",
        );

        assert!(
            self.validation.max_inheritance_depth > 0
                && self.validation.max_inheritance_depth <= 128,
            "max_inheritance_depth must be in range 1..=128 after validation",
        );

        Ok(())
    }

    pub fn output_path(&self) -> &Path {
        &self.compiler.output_directory
    }

    pub fn cache_path(&self) -> &Path {
        &self.compiler.cache_directory
    }

    pub fn vendor_path(&self) -> &Path {
        &self.vendor.vendor_directory
    }

    pub fn all_directories(&self) -> Vec<PathBuf> {
        let mut directories = Vec::with_capacity(
            self.paths.primary_templates.len()
                + self.paths.app_templates.len()
                + 1,
        );

        for directory in &self.paths.primary_templates {
            directories.push(directory.clone());
        }

        for app in &self.paths.app_templates {
            let templates = app.join("templates");

            if templates.exists() {
                directories.push(templates);
            }
        }

        let vendor = &self.vendor.vendor_directory;

        if vendor.exists() {
            directories.push(vendor.clone());
        }

        assert!(
            !directories.is_empty(),
            "all_directories must produce at least one directory",
        );

        directories
    }
}

fn discover_templates(project: &Path) -> Vec<PathBuf> {
    assert!(
        project.is_dir(),
        "project_directory must be a directory: {:?}",
        project,
    );

    let mut directories = Vec::with_capacity(16);
    let mut count: u32 = 0;

    for entry in WalkDir::new(project)
        .follow_links(false)
        .max_depth(4)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();

            !EXCLUDED_DIRECTORY_SEGMENTS
                .iter()
                .any(|segment| name == *segment)
        })
    {
        count += 1;

        assert!(
            count <= PROJECT_WALK_ENTRIES_MAX,
            "discover_templates walk exceeded {} entries",
            PROJECT_WALK_ENTRIES_MAX,
        );

        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        if !entry.path().is_dir() {
            continue;
        }

        if entry.file_name() != "templates" {
            continue;
        }

        let has_html = WalkDir::new(entry.path())
            .follow_links(false)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .any(|entry| {
                entry.path()
                    .extension()
                    .is_some_and(|extension| extension == "html")
            });

        if has_html {
            directories.push(entry.path().to_path_buf());
        }
    }

    directories.sort();
    directories
}

fn discover_environment(project: &Path) -> Option<PathBuf> {
    assert!(
        project.is_dir(),
        "project_directory must be a directory: {:?}",
        project,
    );

    if let Ok(value) = std::env::var("VIRTUAL_ENV") {
        let path = PathBuf::from(&value);

        if path.is_dir() {
            return Some(path);
        }
    }

    for name in VENV_PROBE_NAMES {
        let path = project.join(name);

        if path.is_dir() {
            return Some(path);
        }
    }

    None
}
