use std::path::PathBuf;

use anyhow::Result;
use rustc_hash::{FxHashMap, FxHashSet};
use walkdir::WalkDir;

use crate::config::Config;
use compiler::ast::AstNode;


const TEMPLATE_WALK_ENTRIES_MAX: u32 = 500_000;
const TEMPLATE_COUNT_MAX: u32 = 100_000;
const DEPENDENCY_EXTRACT_ITERATIONS_MAX: u32 = 500_000;
const DEPENDENCY_GRAPH_ITERATIONS_MAX: u32 = 500_000;
const ENTRY_TEMPLATE_ITERATIONS_MAX: u32 = 500_000;

const EXCLUDED_DIRECTORY_SEGMENTS: &[&str] = &[
    ".venv",
    ".git",
    "__pycache__",
    "docs",
    "node_modules",
    "test_project",
];

#[derive(Debug, Clone)]
pub struct TemplateIndex {
    pub templates: FxHashMap<String, PathBuf>,
    pub path_to_name: FxHashMap<PathBuf, String>,
}

#[derive(Debug, Clone)]
pub struct DependencyGraph {
    pub dependencies: FxHashMap<String, Vec<Dependency>>,
    pub dependents: FxHashMap<String, FxHashSet<String>>,
}

#[derive(Debug, Clone)]
pub enum DependencyType {
    Extends,
    Include,
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub target: String,
    pub r#type: DependencyType,
}

impl Default for TemplateIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateIndex {
    pub fn new() -> Self {
        Self {
            templates: FxHashMap::default(),
            path_to_name: FxHashMap::default(),
        }
    }
}

pub struct TemplateDiscovery<'a> {
    config: &'a Config,
}

impl<'a> TemplateDiscovery<'a> {
    pub fn new(config: &'a Config) -> Self {
        assert!(
            !config.paths.primary_templates.is_empty(),
            "primary_templates must not be empty for discovery",
        );

        Self { config }
    }

    pub fn scan(&self) -> Result<TemplateIndex> {
        let mut index = TemplateIndex::new();
        let directories = self.config.all_directories();

        assert!(
            !directories.is_empty(),
            "template directories must not be empty for scan",
        );

        for directory in &directories {
            if !directory.exists() {
                continue;
            }

            let mut count: u32 = 0;

            for entry in WalkDir::new(directory)
                .follow_links(false)
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
                    count <= TEMPLATE_WALK_ENTRIES_MAX,
                    "template walk exceeded {} entries for {:?}",
                    TEMPLATE_WALK_ENTRIES_MAX,
                    directory,
                );

                let entry = match entry {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };

                let path = entry.path();

                if !path.is_file() {
                    continue;
                }

                if path
                    .extension()
                    .is_none_or(|extension| extension != "html")
                {
                    continue;
                }

                let relative = match path.strip_prefix(directory) {
                    Ok(relative) => relative,
                    Err(_) => continue,
                };

                let name = relative.to_string_lossy().replace('\\', "/");

                let full = path.to_path_buf();

                index.path_to_name.insert(full.clone(), name.clone());

                index.templates.entry(name).or_insert(full);
            }
        }

        let count = u32::try_from(index.templates.len()).expect("template count must fit in u32");

        assert!(
            count <= TEMPLATE_COUNT_MAX,
            "template count {} exceeds maximum {}",
            count,
            TEMPLATE_COUNT_MAX,
        );

        Ok(index)
    }

    pub fn dependencies(&self, index: &TemplateIndex) -> Result<DependencyGraph> {
        let count = u32::try_from(index.templates.len()).expect("template count must fit in u32");

        assert!(
            count <= TEMPLATE_COUNT_MAX,
            "template count exceeds maximum for dependency graph",
        );

        let mut dependencies: FxHashMap<String, Vec<Dependency>> = FxHashMap::default();

        let mut dependents: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();

        let mut iterations: u32 = 0;

        for (name, path) in &index.templates {
            iterations += 1;

            assert!(
                iterations <= DEPENDENCY_GRAPH_ITERATIONS_MAX,
                "dependencies exceeded {} iterations",
                DEPENDENCY_GRAPH_ITERATIONS_MAX,
            );

            let content = match std::fs::read_to_string(path) {
                Ok(content) => content,
                Err(_) => continue,
            };

            let deps = extract(&content);

            for dep in &deps {
                dependents
                    .entry(dep.target.clone())
                    .or_default()
                    .insert(name.clone());
            }

            dependencies.insert(name.clone(), deps);
        }

        Ok(DependencyGraph {
            dependencies,
            dependents,
        })
    }

    pub fn entries(&self, index: &TemplateIndex, graph: &DependencyGraph) -> Vec<String> {
        let count = u32::try_from(index.templates.len()).expect("template count must fit in u32");

        assert!(
            count <= TEMPLATE_COUNT_MAX,
            "template count exceeds maximum for entry discovery",
        );

        let vendor = self.config.vendor_path();
        let mut seen = FxHashSet::default();
        let mut entries = Vec::new();
        let mut iterations: u32 = 0;

        if self.config.entry_templates.auto_discover {
            for (name, path) in &index.templates {
                iterations += 1;

                assert!(
                    iterations <= ENTRY_TEMPLATE_ITERATIONS_MAX,
                    "entries exceeded {} iterations",
                    ENTRY_TEMPLATE_ITERATIONS_MAX,
                );

                let depended = graph
                    .dependents
                    .get(name)
                    .is_some_and(|dependents| !dependents.is_empty());

                if depended {
                    continue;
                }

                let is_vendor = path.starts_with(vendor);

                if is_vendor {
                    let has_extends = graph.dependencies.get(name).is_some_and(|deps| {
                        deps.iter()
                            .any(|d| matches!(d.r#type, DependencyType::Extends))
                    });

                    if !has_extends {
                        continue;
                    }
                }

                if seen.insert(name.clone()) {
                    entries.push(name.clone());
                }
            }
        }

        for explicit in &self.config.entry_templates.explicit {
            if seen.insert(explicit.clone()) {
                entries.push(explicit.clone());
            }
        }

        entries.sort();
        entries
    }
}

fn extract(content: &str) -> Vec<Dependency> {
    assert!(
        u32::try_from(content.len()).is_ok(),
        "content length must fit in u32 for extract",
    );

    if content.is_empty() {
        return Vec::new();
    }

    let parsed = match compiler::parser::parse(content) {
        Ok(output) => output,
        Err(_) => return Vec::new(),
    };

    let mut dependencies = Vec::with_capacity(4);
    let mut stack: Vec<&[AstNode]> = vec![&parsed.nodes];
    let mut iterations: u32 = 0;

    while let Some(nodes) = stack.pop() {
        for node in nodes {
            iterations += 1;

            assert!(
                iterations <= DEPENDENCY_EXTRACT_ITERATIONS_MAX,
                "extract exceeded {} iterations",
                DEPENDENCY_EXTRACT_ITERATIONS_MAX,
            );

            match node {
                AstNode::Extends(extends) => {
                    dependencies.push(Dependency {
                        target: extends.parent_path.clone(),
                        r#type: DependencyType::Extends,
                    });
                }
                AstNode::Include(include) => {
                    dependencies.push(Dependency {
                        target: include.path.clone(),
                        r#type: DependencyType::Include,
                    });
                }
                _ => {}
            }

            node.push_child_slices(&mut stack);
        }
    }

    assert!(
        iterations <= DEPENDENCY_EXTRACT_ITERATIONS_MAX,
        "extract post-condition: iterations exceeded maximum",
    );

    dependencies
}
