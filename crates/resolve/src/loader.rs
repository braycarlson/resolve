use std::path::{Path, PathBuf};

use rustc_hash::FxHashMap;
use walkdir::WalkDir;

use crate::discovery::TemplateIndex;
use compiler::error::CompileError;
use compiler::resolver::TemplateLoader;

const VENDOR_ENTRIES_MAX: u32 = 100_000;
const VENDOR_WALK_ITERATIONS_MAX: u32 = 500_000;

#[cold]
#[inline(never)]
fn error_read(path: &Path, error: std::io::Error) -> CompileError {
    CompileError::TemplateRead {
        path: path.display().to_string(),
        message: error.to_string(),
    }
}

pub struct VendorIndex {
    lookup: FxHashMap<String, (PathBuf, u32)>,
}

impl VendorIndex {
    pub fn build(path: &Path) -> Self {
        if !path.exists() {
            return Self {
                lookup: FxHashMap::default(),
            };
        }

        assert!(path.is_dir(), "vendor_path must be a directory: {:?}", path,);

        let mut lookup: FxHashMap<String, (PathBuf, u32)> = FxHashMap::default();

        let mut iterations: u32 = 0;

        for entry in WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            iterations += 1;

            assert!(
                iterations <= VENDOR_WALK_ITERATIONS_MAX,
                "vendor walk exceeded {} iterations",
                VENDOR_WALK_ITERATIONS_MAX,
            );

            if !entry.path().is_file() {
                continue;
            }

            if entry
                .path()
                .extension()
                .is_none_or(|extension| extension != "html")
            {
                continue;
            }

            let relative = match entry.path().strip_prefix(path) {
                Ok(relative) => relative,
                Err(_) => continue,
            };

            let name = relative.to_string_lossy().replace('\\', "/");

            let depth =
                u32::try_from(name.matches('/').count()).expect("path depth must fit in u32");

            let suffixes = suffixes(&name);

            for suffix in suffixes {
                let existing = lookup.get(suffix);

                let should_insert = match existing {
                    Some((_, existing_depth)) => depth < *existing_depth,
                    None => true,
                };

                if should_insert {
                    lookup.insert(suffix.to_string(), (entry.path().to_path_buf(), depth));
                }
            }
        }

        let count = u32::try_from(lookup.len()).expect("vendor lookup length must fit in u32");

        assert!(
            count <= VENDOR_ENTRIES_MAX,
            "vendor entries {} exceed maximum {}",
            count,
            VENDOR_ENTRIES_MAX,
        );

        Self { lookup }
    }

    pub fn find(&self, name: &str) -> Option<PathBuf> {
        assert!(
            !name.is_empty(),
            "template_name must not be empty for vendor lookup",
        );

        self.lookup.get(name).map(|(path, _)| path.clone())
    }
}

fn suffixes(path: &str) -> Vec<&str> {
    assert!(
        !path.is_empty(),
        "path must not be empty for suffix extraction",
    );

    let mut result = Vec::with_capacity(8);
    result.push(path);

    let mut iterations: u32 = 0;

    for (index, byte) in path.bytes().enumerate() {
        iterations += 1;

        assert!(
            iterations <= VENDOR_WALK_ITERATIONS_MAX,
            "suffixes exceeded {} iterations",
            VENDOR_WALK_ITERATIONS_MAX,
        );

        if byte == b'/' && index + 1 < path.len() {
            result.push(&path[index + 1..]);
        }
    }

    assert!(
        !result.is_empty(),
        "suffixes must contain at least the original path",
    );

    result
}

pub struct FsTemplateLoader<'a> {
    index: &'a TemplateIndex,
    vendor: &'a VendorIndex,
}

impl<'a> FsTemplateLoader<'a> {
    pub fn new(index: &'a TemplateIndex, vendor: &'a VendorIndex) -> Self {
        Self { index, vendor }
    }
}

impl TemplateLoader for FsTemplateLoader<'_> {
    fn load(&self, name: &str) -> Result<Option<(PathBuf, String)>, CompileError> {
        assert!(!name.is_empty(), "template name must not be empty",);

        let path = resolve_path(name, self.index, self.vendor);

        let Some(path) = path else {
            return Ok(None);
        };

        assert!(path.is_file(), "resolved path must be a file: {:?}", path,);

        let content = std::fs::read_to_string(&path).map_err(|error| error_read(&path, error))?;

        Ok(Some((path, content)))
    }

    fn load_excluding(
        &self,
        name: &str,
        exclude: Option<&Path>,
    ) -> Result<Option<(PathBuf, String)>, CompileError> {
        assert!(!name.is_empty(), "template name must not be empty",);

        let path = resolve_excluding(name, self.index, exclude, self.vendor);

        let Some(path) = path else {
            return Ok(None);
        };

        assert!(path.is_file(), "resolved path must be a file: {:?}", path,);

        let content = std::fs::read_to_string(&path).map_err(|error| error_read(&path, error))?;

        Ok(Some((path, content)))
    }
}

fn resolve_path(name: &str, index: &TemplateIndex, vendor: &VendorIndex) -> Option<PathBuf> {
    assert!(!name.is_empty(), "template_name must not be empty",);

    if let Some(path) = index.templates.get(name) {
        return Some(path.clone());
    }

    vendor.find(name)
}

fn resolve_excluding(
    name: &str,
    index: &TemplateIndex,
    exclude: Option<&Path>,
    vendor: &VendorIndex,
) -> Option<PathBuf> {
    assert!(!name.is_empty(), "template_name must not be empty",);

    if let Some(path) = index.templates.get(name) {
        let dominated = match exclude {
            Some(excluded) => path.as_path() == excluded,
            None => false,
        };

        if !dominated {
            return Some(path.clone());
        }
    }

    vendor.find(name)
}

pub fn find_in_vendor(path: &str, vendor: &VendorIndex) -> Option<PathBuf> {
    assert!(!path.is_empty(), "template_path must not be empty",);

    vendor.find(path)
}
