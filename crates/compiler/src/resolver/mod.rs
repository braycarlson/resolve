pub mod inclusion;
pub mod inheritance;

use std::path::{Path, PathBuf};

use crate::error::CompileError;

pub struct ResolveLimits {
    pub max_include_depth: u32,
    pub max_inheritance_depth: u32,
}

pub trait TemplateLoader {
    fn load(&self, name: &str) -> Result<Option<(PathBuf, String)>, CompileError>;

    fn load_excluding(
        &self,
        name: &str,
        exclude: Option<&Path>,
    ) -> Result<Option<(PathBuf, String)>, CompileError>;
}
