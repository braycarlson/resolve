use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::reporter::Reporter;
use anyhow::{Context, Result};
use rustc_hash::FxHashSet;
use walkdir::WalkDir;


const VIRTUAL_ENVIRONMENT_PROBE_NAMES: &[&str] = &[".venv", "venv"];

const SITE_PACKAGES_ENTRIES_MAX: u32 = 10_000;
const TEMPLATES_PER_PACKAGE_MAX: u32 = 50_000;
const TEMPLATE_DIRECTORY_WALK_MAX: u32 = 100_000;
const STALE_CHECK_WALK_MAX: u32 = 500_000;
const COUNT_WALK_MAX: u32 = 500_000;
const PTH_SCAN_ENTRIES_MAX: u32 = 10_000;
const PTH_LINES_MAX: u32 = 50_000;
const PTH_SUB_ENTRIES_MAX: u32 = 50_000;
const DIST_INFO_ENTRIES_MAX: u32 = 10_000;

const EXCLUDED_PREFIXES: &[&str] = &[
    "_",
    "pip",
    "pkg_resources",
    "setuptools",
    "wheel",
    "distutils",
];

pub struct VendorManager<'a> {
    config: &'a Config,
    reporter: &'a Reporter,
}

impl<'a> VendorManager<'a> {
    pub fn new(config: &'a Config, reporter: &'a Reporter) -> Self {
        Self { config, reporter }
    }

    pub fn vendor_exists(&self) -> bool {
        self.config.vendor_path().exists()
    }

    pub fn is_vendor_stale(&self) -> Result<bool> {
        let path = self.config.vendor_path();

        if !path.exists() {
            return Ok(true);
        }

        assert!(
            path.is_dir(),
            "vendor_path must be a directory: {:?}",
            path,
        );

        let sources = self.discover()?;

        if sources.is_empty() {
            return Ok(false);
        }

        for source in &sources {
            if is_source_stale(source, path)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn discover(&self) -> Result<Vec<VendorSource>> {
        if !self.config.vendor.auto_detect {
            return Ok(Vec::new());
        }

        let packages = find_site_packages(
            &self.config.vendor,
            &self.config.paths.primary_templates,
            self.reporter,
        )?;

        let Some(packages) = packages else {
            self.reporter.debug("No site-packages directory found");
            return Ok(Vec::new());
        };

        self.reporter.debug(&format!(
            "Scanning site-packages: {:?}",
            packages,
        ));

        scan_packages(&packages, self.reporter)
    }

    pub fn sync(&self) -> Result<u32> {
        assert!(
            self.config.vendor.auto_detect
                || !self.config.vendor.vendor_directory.as_os_str().is_empty(),
            "vendor configuration must be valid for sync",
        );

        let path = self.config.vendor_path();
        let sources = self.discover()?;

        if sources.is_empty() {
            self.reporter.debug("No vendor packages with templates found");
            return Ok(0);
        }

        fs::create_dir_all(path)?;

        let mut copied: u32 = 0;

        for source in &sources {
            copied += sync_source(source, path, self.reporter)?;
        }

        assert!(
            path.exists(),
            "vendor directory must exist after sync",
        );

        Ok(copied)
    }

    pub fn status(&self) -> Result<()> {
        let path = self.config.vendor_path();

        if !path.exists() {
            self.reporter.info(
                "Vendor directory does not exist. Run 'vendor sync' to create it.",
            );

            return Ok(());
        }

        let sources = self.discover()?;

        self.reporter.info(&format!(
            "Discovered {} vendor packages:",
            sources.len(),
        ));

        for source in &sources {
            self.reporter.info(&format!(
                "  {} ({} template directories)",
                source.name,
                source.directories.len(),
            ));
        }

        let stale = self.is_vendor_stale()?;

        if stale {
            self.reporter.info(
                "Vendor templates are stale. Run 'vendor sync' to update.",
            );
        } else {
            self.reporter.info("Vendor templates are up to date.");
        }

        let count = count_templates(path);

        self.reporter.info(&format!(
            "Vendor contains {} templates",
            count,
        ));

        Ok(())
    }
}

#[derive(Debug)]
pub struct VendorSource {
    pub name: String,
    pub path: PathBuf,
    pub directories: Vec<PathBuf>,
}

fn find_environment(
    config: &crate::config::VendorConfig,
    directories: &[PathBuf],
    reporter: &Reporter,
) -> Option<PathBuf> {
    assert!(
        !directories.is_empty(),
        "template_directories must not be empty for venv search",
    );

    if let Ok(value) = std::env::var("VIRTUAL_ENV") {
        let path = PathBuf::from(&value);

        if path.is_dir() {
            reporter.debug(&format!("Using VIRTUAL_ENV: {:?}", path));
            return Some(path);
        }
    }

    if let Some(ref path) = config.virtual_environment_path {
        if path.is_dir() {
            reporter.debug(&format!(
                "Using configured virtual_environment_path: {:?}",
                path,
            ));
            return Some(path.clone());
        }

        reporter.warn(&format!(
            "Configured virtual_environment_path does not exist: {:?}",
            path,
        ));
    }

    for name in VIRTUAL_ENVIRONMENT_PROBE_NAMES {
        let path = PathBuf::from(name);

        if path.is_dir() {
            reporter.debug(&format!("Probed virtual environment at: {:?}", path));
            return Some(path);
        }
    }

    for directory in directories {
        if let Some(root) = directory.parent() {
            for name in VIRTUAL_ENVIRONMENT_PROBE_NAMES {
                let path = root.join(name);

                if path.is_dir() {
                    reporter.debug(&format!(
                        "Probed virtual environment relative to templates at: {:?}",
                        path,
                    ));

                    return Some(path);
                }
            }
        }
    }

    None
}

fn find_site_packages(
    config: &crate::config::VendorConfig,
    directories: &[PathBuf],
    reporter: &Reporter,
) -> Result<Option<PathBuf>> {
    assert!(
        !directories.is_empty(),
        "template_directories must not be empty for site-packages search",
    );

    let environment =
        match find_environment(config, directories, reporter) {
            Some(environment) => environment,
            None => return Ok(None),
        };

    assert!(
        environment.is_dir(),
        "resolved virtual environment path must be a directory: {:?}",
        environment,
    );

    let windows = environment.join("Lib").join("site-packages");

    if windows.is_dir() {
        return Ok(Some(windows));
    }

    let lib = environment.join("lib");

    if !lib.is_dir() {
        return Ok(None);
    }

    let entries = fs::read_dir(&lib)
        .context("failed to read virtual environment lib directory")?;

    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if name.starts_with("python") && entry.path().is_dir() {
            let packages = entry.path().join("site-packages");

            if packages.is_dir() {
                return Ok(Some(packages));
            }
        }
    }

    Ok(None)
}

fn skip_package(name: &str) -> bool {
    assert!(
        !name.is_empty(),
        "package name must not be empty",
    );

    if name.ends_with(".dist-info")
        || name.ends_with(".egg-info")
        || name.ends_with(".egg-link")
        || name.ends_with(".pth")
    {
        return true;
    }

    if EXCLUDED_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
    {
        return true;
    }

    false
}

fn scan_packages(
    packages: &Path,
    reporter: &Reporter,
) -> Result<Vec<VendorSource>> {
    assert!(
        packages.is_dir(),
        "site_packages must be a directory: {:?}",
        packages,
    );

    let entries = fs::read_dir(packages)
        .context("failed to read site-packages")?;

    let mut sources = Vec::with_capacity(16);
    let mut seen: FxHashSet<String> = FxHashSet::default();

    for (index, entry) in entries.enumerate() {
        let count = u32::try_from(index)
            .expect("site-packages entry index must fit in u32");

        assert!(
            count < SITE_PACKAGES_ENTRIES_MAX,
            "site-packages entries exceeded maximum of {}",
            SITE_PACKAGES_ENTRIES_MAX,
        );

        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let name = match path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => continue,
        };

        if skip_package(&name) {
            continue;
        }

        let directories = find_templates(&path);

        if directories.is_empty() {
            continue;
        }

        reporter.debug(&format!(
            "Found vendor package '{}' with {} template directories",
            name,
            directories.len(),
        ));

        seen.insert(name.clone());

        sources.push(VendorSource {
            name,
            path,
            directories,
        });
    }

    let editable = scan_pth(packages, reporter)?;

    for source in editable {
        if !seen.contains(&source.name) {
            reporter.debug(&format!(
                "Found editable vendor package '{}' with {} template directories",
                source.name,
                source.directories.len(),
            ));

            seen.insert(source.name.clone());
            sources.push(source);
        }
    }

    let dist = scan_editable(packages, reporter)?;

    for source in dist {
        if !seen.contains(&source.name) {
            reporter.debug(&format!(
                "Found editable vendor package '{}' with {} template directories (dist-info)",
                source.name,
                source.directories.len(),
            ));

            seen.insert(source.name.clone());
            sources.push(source);
        }
    }

    assert!(
        u32::try_from(sources.len()).is_ok(),
        "vendor source count must fit in u32",
    );

    Ok(sources)
}

fn find_templates(package: &Path) -> Vec<PathBuf> {
    assert!(
        package.is_dir(),
        "package must be a directory: {:?}",
        package,
    );

    let mut directories = Vec::with_capacity(8);
    let mut count: u32 = 0;

    for entry in WalkDir::new(package)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            name != "__pycache__"
                && name != ".git"
                && name != "node_modules"
                && name != "migrations"
        })
    {
        count += 1;

        assert!(
            count <= TEMPLATE_DIRECTORY_WALK_MAX,
            "find_templates walk exceeded {} iterations",
            TEMPLATE_DIRECTORY_WALK_MAX,
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

    directories
}

fn scan_pth(
    packages: &Path,
    reporter: &Reporter,
) -> Result<Vec<VendorSource>> {
    assert!(
        packages.is_dir(),
        "site_packages must be a directory for pth scan: {:?}",
        packages,
    );

    let mut sources = Vec::with_capacity(4);
    let mut iterations: u32 = 0;

    let entries = fs::read_dir(packages)
        .context("failed to read site-packages for .pth files")?;

    for entry in entries {
        iterations += 1;

        assert!(
            iterations <= PTH_SCAN_ENTRIES_MAX,
            "scan_pth exceeded {} iterations",
            PTH_SCAN_ENTRIES_MAX,
        );

        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let extension = match path.extension() {
            Some(extension) => extension.to_string_lossy().to_string(),
            None => continue,
        };

        if extension != "pth" {
            continue;
        }

        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => continue,
        };

        let mut count: u32 = 0;

        for line in content.lines() {
            count += 1;

            assert!(
                count <= PTH_LINES_MAX,
                "scan_pth line iteration exceeded {} for {:?}",
                PTH_LINES_MAX,
                path,
            );

            let line = line.trim();

            if line.is_empty()
                || line.starts_with('#')
                || line.starts_with("import")
            {
                continue;
            }

            let source = PathBuf::from(line);

            if !source.is_dir() {
                continue;
            }

            let sub_entries = match fs::read_dir(&source) {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            let mut sub_count: u32 = 0;

            for sub_entry in sub_entries {
                sub_count += 1;

                assert!(
                    sub_count <= PTH_SUB_ENTRIES_MAX,
                    "scan_pth sub_entry iteration exceeded {} for {:?}",
                    PTH_SUB_ENTRIES_MAX,
                    source,
                );

                let sub_entry = match sub_entry {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };

                let sub_path = sub_entry.path();

                if !sub_path.is_dir() {
                    continue;
                }

                let name = match sub_path.file_name() {
                    Some(name) => name.to_string_lossy().to_string(),
                    None => continue,
                };

                if skip_package(&name) {
                    continue;
                }

                let directories = find_templates(&sub_path);

                if directories.is_empty() {
                    continue;
                }

                reporter.debug(&format!(
                    "Found editable package '{}' via .pth file",
                    name,
                ));

                sources.push(VendorSource {
                    name,
                    path: sub_path,
                    directories,
                });
            }
        }
    }

    Ok(sources)
}

fn scan_editable(
    packages: &Path,
    reporter: &Reporter,
) -> Result<Vec<VendorSource>> {
    assert!(
        packages.is_dir(),
        "site_packages must be a directory for dist-info scan: {:?}",
        packages,
    );

    let mut sources = Vec::with_capacity(4);
    let mut iterations: u32 = 0;

    let entries = fs::read_dir(packages)
        .context("failed to read site-packages for dist-info")?;

    for entry in entries {
        iterations += 1;

        assert!(
            iterations <= DIST_INFO_ENTRIES_MAX,
            "scan_editable exceeded {} iterations",
            DIST_INFO_ENTRIES_MAX,
        );

        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let name = match path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => continue,
        };

        if !name.ends_with(".dist-info") {
            continue;
        }

        let manifest = path.join("direct_url.json");

        if !manifest.is_file() {
            continue;
        }

        let content = match fs::read_to_string(&manifest) {
            Ok(content) => content,
            Err(_) => continue,
        };

        let url: serde_json::Value =
            match serde_json::from_str(&content) {
                Ok(value) => value,
                Err(_) => continue,
            };

        let is_editable = url
            .get("dir_info")
            .and_then(|info| info.get("editable"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false);

        if !is_editable {
            continue;
        }

        let raw_url = match url.get("url").and_then(|value| value.as_str()) {
            Some(url) => url,
            None => continue,
        };

        let source = match url_to_path(raw_url) {
            Some(path) => path,
            None => continue,
        };

        if !source.is_dir() {
            continue;
        }

        let package = read_top_level(&path)
            .unwrap_or_else(|| extract_name(&name));

        if package.is_empty() {
            continue;
        }

        if skip_package(&package) {
            continue;
        }

        let target = source.join(&package);

        if !target.is_dir() {
            continue;
        }

        let directories = find_templates(&target);

        if directories.is_empty() {
            continue;
        }

        reporter.debug(&format!(
            "Found editable package '{}' via dist-info",
            package,
        ));

        sources.push(VendorSource {
            name: package,
            path: target,
            directories,
        });
    }

    assert!(
        u32::try_from(sources.len()).is_ok(),
        "editable source count must fit in u32",
    );

    Ok(sources)
}

fn url_to_path(url: &str) -> Option<PathBuf> {
    assert!(
        !url.is_empty(),
        "url must not be empty",
    );

    let stripped = url.strip_prefix("file://")?;

    #[cfg(target_os = "windows")]
    let raw = stripped
        .strip_prefix('/')
        .unwrap_or(stripped);

    #[cfg(not(target_os = "windows"))]
    let raw = stripped;

    let path = PathBuf::from(raw);

    if path.is_absolute() {
        Some(path)
    } else {
        None
    }
}

fn read_top_level(dist_info: &Path) -> Option<String> {
    assert!(
        dist_info.is_dir(),
        "dist_info must be a directory: {:?}",
        dist_info,
    );

    let path = dist_info.join("top_level.txt");
    let content = fs::read_to_string(path).ok()?;
    let name = content.lines().next()?.trim().to_string();

    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn extract_name(directory: &str) -> String {
    assert!(
        !directory.is_empty(),
        "directory must not be empty",
    );

    directory
        .strip_suffix(".dist-info")
        .and_then(|stripped| stripped.split('-').next())
        .unwrap_or("")
        .to_string()
}

fn sync_source(
    source: &VendorSource,
    vendor: &Path,
    reporter: &Reporter,
) -> Result<u32> {
    assert!(
        vendor.is_dir(),
        "vendor must exist before sync: {:?}",
        vendor,
    );

    assert!(
        !source.name.is_empty(),
        "source name must not be empty",
    );

    let mut copied: u32 = 0;

    for directory in &source.directories {
        for (index, entry) in WalkDir::new(directory)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                let name = entry.file_name().to_string_lossy();
                name != "__pycache__" && name != ".git"
            })
            .enumerate()
        {
            let count = u32::try_from(index)
                .expect("template index must fit in u32");

            assert!(
                count < TEMPLATES_PER_PACKAGE_MAX,
                "templates in package '{}' exceeded maximum of {}",
                source.name,
                TEMPLATES_PER_PACKAGE_MAX,
            );

            let entry = entry?;
            let source = entry.path();

            if !source.is_file() {
                continue;
            }

            if !source
                .extension()
                .is_some_and(|extension| extension == "html")
            {
                continue;
            }

            let relative =
                match source.strip_prefix(directory) {
                    Ok(relative) => relative,
                    Err(_) => continue,
                };

            let target = vendor.join(relative);

            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::copy(source, &target)?;
            copied += 1;
        }
    }

    if copied > 0 {
        reporter.info(&format!(
            "  Synced {} templates from '{}'",
            copied,
            source.name,
        ));
    }

    Ok(copied)
}

fn is_source_stale(
    source: &VendorSource,
    vendor: &Path,
) -> Result<bool> {
    assert!(
        !source.name.is_empty(),
        "source name must not be empty",
    );

    assert!(
        vendor.is_dir(),
        "vendor must be a directory for stale check",
    );

    let mut count: u32 = 0;

    for directory in &source.directories {
        for entry in WalkDir::new(directory)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                let name = entry.file_name().to_string_lossy();
                name != "__pycache__" && name != ".git"
            })
        {
            count += 1;

            assert!(
                count <= STALE_CHECK_WALK_MAX,
                "is_source_stale walk exceeded {} iterations",
                STALE_CHECK_WALK_MAX,
            );

            let entry = entry?;
            let source = entry.path();

            if !source.is_file() {
                continue;
            }

            if !source
                .extension()
                .is_some_and(|extension| extension == "html")
            {
                continue;
            }

            let relative =
                match source.strip_prefix(directory) {
                    Ok(relative) => relative,
                    Err(_) => continue,
                };

            let target = vendor.join(relative);

            if !target.exists() {
                return Ok(true);
            }

            if fs::metadata(source)?.modified()?
                > fs::metadata(&target)?.modified()?
            {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

fn count_templates(vendor: &Path) -> u32 {
    assert!(
        vendor.is_dir(),
        "vendor must be a directory for counting: {:?}",
        vendor,
    );

    let mut count: u32 = 0;

    for (index, entry) in WalkDir::new(vendor)
        .follow_links(false)
        .into_iter()
        .enumerate()
    {
        let walked = u32::try_from(index)
            .expect("count_templates walk index must fit in u32");

        assert!(
            walked <= COUNT_WALK_MAX,
            "count_templates walk exceeded {} iterations",
            COUNT_WALK_MAX,
        );

        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "html")
        {
            count += 1;
        }
    }

    count
}

pub fn sync(config: &Config, reporter: &Reporter) -> Result<()> {
    assert!(
        !config.vendor.vendor_directory.as_os_str().is_empty(),
        "vendor_directory must not be empty for sync",
    );

    let manager = VendorManager::new(config, reporter);
    let count = manager.sync()?;
    reporter.info(&format!("Synced {} vendor templates", count));

    Ok(())
}

pub fn status(config: &Config, reporter: &Reporter) -> Result<()> {
    assert!(
        !config.vendor.vendor_directory.as_os_str().is_empty(),
        "vendor_directory must not be empty for status",
    );

    let manager = VendorManager::new(config, reporter);
    manager.status()
}
