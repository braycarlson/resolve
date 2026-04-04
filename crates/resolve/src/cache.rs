use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::time::SystemTime;

use anyhow::Result;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::Config;


const FILE_READ_ITERATIONS_MAX: u32 = 2_000_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileManifest {
    pub files: FxHashMap<String, FileEntry>,
    pub resolve_templates: FxHashSet<String>,
    pub last_compilation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub hash: String,
    pub size: u64,
    pub modified: u64,
}

pub struct CompileCache {
    manifest: CompileManifest,
    directory: std::path::PathBuf,
}

impl CompileCache {
    pub fn new(config: &Config) -> Result<Self> {
        assert!(
            !config.cache_path().as_os_str().is_empty(),
            "cache_path must not be empty",
        );

        let directory = config.cache_path().to_path_buf();

        if !directory.exists() {
            fs::create_dir_all(&directory)?;
        }

        assert!(
            directory.exists(),
            "cache directory must exist after creation: {:?}",
            directory,
        );

        let path = directory.join("cache.json");

        let manifest = if path.exists() {
            let content = fs::read_to_string(&path)?;

            match serde_json::from_str(&content) {
                Ok(parsed) => parsed,
                Err(_) => {
                    let _ = fs::remove_file(&path);
                    Self::empty()
                }
            }
        } else {
            Self::empty()
        };

        Ok(Self {
            manifest,
            directory,
        })
    }

    pub fn manifest_mut(&mut self) -> &mut CompileManifest {
        assert!(
            self.directory.exists(),
            "cache directory must exist for manifest_mut: {:?}",
            self.directory,
        );

        assert!(
            !self.directory.as_os_str().is_empty(),
            "directory must not be empty in manifest_mut",
        );

        &mut self.manifest
    }

    fn empty() -> CompileManifest {
        let manifest = CompileManifest {
            files: FxHashMap::default(),
            resolve_templates: FxHashSet::default(),
            last_compilation: String::new(),
        };

        assert!(
            manifest.files.is_empty(),
            "empty manifest must have no files",
        );

        assert!(
            manifest.resolve_templates.is_empty(),
            "empty manifest must have no compiled templates",
        );

        manifest
    }

    pub fn needs_recompile(
        &self,
        name: &str,
        path: &Path,
    ) -> bool {
        assert!(
            !name.is_empty(),
            "template_name must not be empty",
        );

        assert!(
            path.is_file(),
            "template_path must be a file: {:?}",
            path,
        );

        let Some(cached) = self.manifest.files.get(name) else {
            return true;
        };

        let Ok(current) = Self::entry(path) else {
            return true;
        };

        cached.hash != current.hash
            || cached.size != current.size
    }

    pub fn mark_compiled(
        &mut self,
        name: &str,
        path: &Path,
    ) -> Result<()> {
        assert!(
            !name.is_empty(),
            "template_name must not be empty",
        );

        let entry = Self::entry(path)?;

        self.manifest
            .files
            .insert(name.to_string(), entry);

        self.manifest
            .resolve_templates
            .insert(name.to_string());

        assert!(
            self.manifest.files.contains_key(name),
            "file entry must exist after mark_compiled",
        );

        Ok(())
    }

    fn entry(path: &Path) -> Result<FileEntry> {
        assert!(
            path.is_file(),
            "entry path must be a file: {:?}",
            path,
        );

        let mut file = fs::File::open(path)?;
        let metadata = file.metadata()?;

        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];
        let mut iterations: u32 = 0;

        loop {
            iterations += 1;

            assert!(
                iterations <= FILE_READ_ITERATIONS_MAX,
                "entry read loop exceeded {} iterations for {:?}",
                FILE_READ_ITERATIONS_MAX,
                path,
            );

            let read = file.read(&mut buffer)?;

            if read == 0 {
                break;
            }

            hasher.update(&buffer[..read]);
        }

        let digest = hasher.finalize();
        let mut hash = String::with_capacity(64);

        for byte in digest.iter() {
            use std::fmt::Write;

            write!(hash, "{:02x}", byte)
                .expect("hex formatting must not fail");
        }

        let modified = metadata
            .modified()?
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system clock must not be before UNIX epoch")
            .as_secs();

        assert!(
            hash.len() == 64,
            "sha256 hash must be exactly 64 hex characters",
        );

        Ok(FileEntry {
            hash,
            size: metadata.len(),
            modified,
        })
    }

    pub fn save(&self) -> Result<()> {
        assert!(
            self.directory.exists(),
            "cache directory must exist before save: {:?}",
            self.directory,
        );

        let content = serde_json::to_string_pretty(&self.manifest)?;
        let path = self.directory.join("cache.json");
        let temp = self.directory.join("cache.json.tmp");

        let mut file = fs::File::create(&temp)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;

        fs::rename(&temp, &path)?;

        assert!(
            path.exists(),
            "manifest file must exist after save: {:?}",
            path,
        );

        Ok(())
    }

    pub fn clean(config: &Config) -> Result<()> {
        let directory = config.cache_path();

        assert!(
            !directory.as_os_str().is_empty(),
            "cache directory path must not be empty",
        );

        if directory.exists() {
            fs::remove_dir_all(directory)?;

            assert!(
                !directory.exists(),
                "cache directory must not exist after clean: {:?}",
                directory,
            );
        }

        Ok(())
    }
}
