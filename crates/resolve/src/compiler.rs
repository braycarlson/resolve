use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use rustc_hash::FxHashSet;

use crate::cache::CompileCache;
use crate::config::Config;
use crate::discovery::{DependencyGraph, TemplateDiscovery, TemplateIndex};
use crate::loader::{FsTemplateLoader, VendorIndex};
use crate::reporter::Reporter;
use crate::swap;
use crate::validator::Validator;
use compiler::resolver::ResolveLimits;

const COMPILE_TEMPLATES_MAX: u32 = 100_000;
const COPY_TEMPLATES_MAX: u32 = 100_000;
const DEPENDENCY_WALK_MAX: u32 = 500_000;

type ExecuteResult = (u32, u32, Vec<(String, String)>, u32);
type BatchResult = (Vec<(String, bool)>, Vec<(String, String)>);

pub struct Compiler<'a> {
    config: &'a Config,
    reporter: &'a Reporter,
}

impl<'a> Compiler<'a> {
    pub fn new(config: &'a Config, reporter: &'a Reporter) -> Self {
        Self { config, reporter }
    }

    pub fn compile_all(&self) -> Result<()> {
        let started = Instant::now();

        let output = self.config.output_path();
        swap::recover_interrupted(output, self.reporter)?;

        self.sync_vendor()?;

        let (index, entries, graph) = self.discover()?;
        let vendor = VendorIndex::build(self.config.vendor_path());

        let remaining = index.templates.len() - entries.len();

        self.reporter.info("Scan");
        self.reporter
            .info(&format!("  {} templates found", index.templates.len(),));
        self.reporter.info(&format!(
            "  {} entry, {} non-compiled\n",
            entries.len(),
            remaining,
        ));

        let (to_compile, mut cache) = self.plan(&index, &entries, &graph, output)?;

        if to_compile.is_empty() {
            self.reporter.info("All templates are up to date.");
            return Ok(());
        }

        let (compiled, failed, failures, copied) =
            self.execute(&to_compile, &index, &vendor, output, &mut cache)?;

        if !failures.is_empty() {
            self.reporter.info(&format!(
                "\n{} template{} failed:",
                failures.len(),
                if failures.len() == 1 { "" } else { "s" },
            ));

            for (name, message) in &failures {
                self.reporter.info(&format!("  {}: {}", name, message));
            }
        }

        let elapsed = started.elapsed();

        self.reporter.info(&format!(
            "\nDone in {:.2}s ({} compiled, {} failed, {} copied)",
            elapsed.as_secs_f64(),
            compiled,
            failed,
            copied,
        ));

        Ok(())
    }

    fn plan(
        &self,
        index: &TemplateIndex,
        entries: &[String],
        graph: &DependencyGraph,
        output: &Path,
    ) -> Result<(Vec<String>, CompileCache)> {
        assert!(
            !entries.is_empty() || index.templates.is_empty(),
            "entry_templates must not be empty when templates exist",
        );

        if !self.config.incremental.enabled {
            CompileCache::clean(self.config)?;
        }

        let cache = CompileCache::new(self.config)?;

        let missing =
            !output.exists() || output.read_dir().map_or(true, |mut d| d.next().is_none());

        let to_compile = if missing {
            entries.to_vec()
        } else {
            self.find_stale(&cache, index, entries, graph)
        };

        let count = u32::try_from(to_compile.len()).expect("to_compile length must fit in u32");

        assert!(
            count <= COMPILE_TEMPLATES_MAX,
            "to_compile count exceeds maximum",
        );

        Ok((to_compile, cache))
    }

    fn execute(
        &self,
        to_compile: &[String],
        index: &TemplateIndex,
        vendor: &VendorIndex,
        output: &Path,
        cache: &mut CompileCache,
    ) -> Result<ExecuteResult> {
        assert!(
            !to_compile.is_empty(),
            "to_compile must not be empty in execute",
        );

        let staging = swap::staging_path(output);

        if staging.exists() {
            fs::remove_dir_all(&staging)?;
        }

        fs::create_dir_all(&staging)?;

        self.reporter
            .info(&format!("Compile ({} templates)", to_compile.len(),));

        let (results, failures) = self.compile_batch(to_compile, index, vendor, &staging, cache)?;

        assert!(
            results.len() == to_compile.len(),
            "results length must match to_compile length",
        );

        let compiled = u32::try_from(results.iter().filter(|(_, success)| *success).count())
            .expect("compiled count must fit in u32");

        let failed = u32::try_from(results.iter().filter(|(_, success)| !*success).count())
            .expect("failed count must fit in u32");

        let names: FxHashSet<&str> = results
            .iter()
            .filter(|(_, success)| *success)
            .map(|(name, _)| name.as_str())
            .collect();

        let remaining = index
            .templates
            .keys()
            .filter(|name| !names.contains(name.as_str()))
            .filter(|name| !staging.join(name).exists())
            .count();

        if remaining > 0 {
            self.reporter
                .info(&format!("\nCopy ({} templates)", remaining,));
        }

        let copied = self.copy_remaining(index, &staging, &results)?;

        swap::swap_staging_to_output(&staging, output, self.reporter)?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must not be before UNIX epoch")
            .as_secs();

        cache.manifest_mut().last_compilation = timestamp.to_string();
        cache.save()?;

        Ok((compiled, failed, failures, copied))
    }

    pub fn compile_single(&self, name: &str) -> Result<()> {
        assert!(!name.is_empty(), "template_name must not be empty",);

        let discovery = TemplateDiscovery::new(self.config);
        let index = discovery.scan()?;

        let normalized = name.replace('\\', "/");

        assert!(
            !normalized.is_empty(),
            "normalized template_name must not be empty",
        );

        let path = index
            .templates
            .get(&normalized)
            .ok_or_else(|| anyhow::anyhow!("Template not found: {}", normalized))?;

        let vendor = VendorIndex::build(self.config.vendor_path());
        let output = self.config.output_path();

        fs::create_dir_all(output)?;

        self.reporter.info(&format!("Compiling {}...", normalized));

        self.compile_template(&normalized, path, &index, &vendor, output)?;

        self.reporter.info(&format!("Compiled: {}", normalized));

        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        let discovery = TemplateDiscovery::new(self.config);
        let index = discovery.scan()?;
        let graph = discovery.dependencies(&index)?;

        let entries = discovery.entries(&index, &graph);
        let vendor = VendorIndex::build(self.config.vendor_path());

        let mut validator = Validator::new();

        let result = validator.validate(&index, &vendor, &entries, self.config.vendor_path())?;

        for error in &result.errors {
            self.reporter.info(error);
        }

        for warning in &result.warnings {
            self.reporter.info(warning);
        }

        if !result.errors.is_empty() {
            return Err(anyhow::anyhow!("Validation failed"));
        }

        Ok(())
    }

    pub fn dry_run(&self) -> Result<()> {
        self.reporter.info("Scanning templates...");

        let discovery = TemplateDiscovery::new(self.config);
        let index = discovery.scan()?;
        let graph = discovery.dependencies(&index)?;
        let entries = discovery.entries(&index, &graph);

        self.reporter
            .info(&format!("  {} templates found", index.templates.len(),));

        self.reporter
            .info(&format!("  {} entry templates", entries.len(),));

        self.reporter
            .info("\nEntry templates that would be compiled:");

        for entry in &entries {
            self.reporter.info(&format!("  {}", entry));
        }

        Ok(())
    }

    fn sync_vendor(&self) -> Result<u32> {
        assert!(
            !self.config.vendor.vendor_directory.as_os_str().is_empty(),
            "vendor_directory must not be empty for sync_vendor",
        );

        let manager = crate::vendor::VendorManager::new(self.config, self.reporter);

        if !manager.vendor_exists() || manager.is_vendor_stale()? {
            self.reporter.info("Vendor");

            let count = manager.sync()?;

            self.reporter
                .info(&format!("  {} templates synced\n", count,));

            assert!(
                self.config.vendor_path().exists() || count == 0,
                "vendor directory must exist after sync when templates were synced",
            );

            return Ok(count);
        }

        self.reporter.info("Vendor");
        self.reporter.info("  Up to date\n");

        Ok(0)
    }

    fn discover(&self) -> Result<(TemplateIndex, Vec<String>, DependencyGraph)> {
        assert!(
            !self.config.paths.primary_templates.is_empty(),
            "primary_templates must not be empty for discovery",
        );

        let discovery = TemplateDiscovery::new(self.config);
        let index = discovery.scan()?;
        let graph = discovery.dependencies(&index)?;
        let entries = discovery.entries(&index, &graph);

        let count = u32::try_from(entries.len()).expect("entry_templates length must fit in u32");

        assert!(
            count <= COMPILE_TEMPLATES_MAX,
            "entry template count {} exceeds maximum {}",
            count,
            COMPILE_TEMPLATES_MAX,
        );

        Ok((index, entries, graph))
    }

    fn find_stale(
        &self,
        cache: &CompileCache,
        index: &TemplateIndex,
        entries: &[String],
        graph: &DependencyGraph,
    ) -> Vec<String> {
        let count = u32::try_from(entries.len()).expect("entry_templates length must fit in u32");

        assert!(
            count <= COMPILE_TEMPLATES_MAX,
            "entry_templates count exceeds maximum in find_stale",
        );

        let mut to_compile = Vec::new();
        let mut iterations: u32 = 0;

        for entry in entries {
            iterations += 1;

            assert!(
                iterations <= COMPILE_TEMPLATES_MAX,
                "find_stale exceeded {} iterations",
                COMPILE_TEMPLATES_MAX,
            );

            if let Some(path) = index.templates.get(entry)
                && cache.needs_recompile(entry, path)
            {
                to_compile.push(entry.clone());
                continue;
            }

            if Self::has_stale(entry, cache, index, graph) {
                to_compile.push(entry.clone());
            }
        }

        to_compile
    }

    fn has_stale(
        name: &str,
        cache: &CompileCache,
        index: &TemplateIndex,
        graph: &DependencyGraph,
    ) -> bool {
        assert!(
            !name.is_empty(),
            "template_name must not be empty in has_stale",
        );

        let mut visited: FxHashSet<&str> = FxHashSet::default();
        let mut stack: Vec<&str> = vec![name];
        let mut iterations: u32 = 0;

        while let Some(current) = stack.pop() {
            iterations += 1;

            assert!(
                iterations <= DEPENDENCY_WALK_MAX,
                "dependency walk exceeded {} iterations",
                DEPENDENCY_WALK_MAX,
            );

            if !visited.insert(current) {
                continue;
            }

            if let Some(deps) = graph.dependencies.get(current) {
                for dep in deps {
                    if let Some(path) = index.templates.get(&dep.target)
                        && cache.needs_recompile(&dep.target, path)
                    {
                        return true;
                    }

                    stack.push(&dep.target);
                }
            }
        }

        false
    }

    fn compile_batch(
        &self,
        to_compile: &[String],
        index: &TemplateIndex,
        vendor: &VendorIndex,
        output: &Path,
        cache: &mut CompileCache,
    ) -> Result<BatchResult> {
        let count = u32::try_from(to_compile.len()).expect("to_compile length must fit in u32");

        assert!(
            count <= COMPILE_TEMPLATES_MAX,
            "compile batch exceeds maximum template count",
        );

        assert!(
            output.exists(),
            "output directory must exist for compile_batch: {:?}",
            output,
        );

        let mut results = Vec::with_capacity(to_compile.len());
        let mut failures: Vec<(String, String)> = Vec::new();
        let mut iterations: u32 = 0;

        for name in to_compile {
            iterations += 1;

            assert!(
                iterations <= COMPILE_TEMPLATES_MAX,
                "compile_batch exceeded {} iterations",
                COMPILE_TEMPLATES_MAX,
            );

            let path = match index.templates.get(name) {
                Some(path) => path,
                None => {
                    let message = "Template not found in index".to_string();
                    self.reporter.info(&format!("  {}", name));
                    self.reporter.error(&format!("    ERROR {}", message,));
                    failures.push((name.clone(), message));
                    results.push((name.clone(), false));
                    continue;
                }
            };

            self.reporter.info(&format!("  {}", name));

            match self.compile_template(name, path, index, vendor, output) {
                Ok(()) => {
                    if let Err(error) = cache.mark_compiled(name, path) {
                        self.reporter
                            .warn(&format!("    WARN Failed to cache: {}", error,));
                    }

                    results.push((name.clone(), true));
                }
                Err(error) => {
                    let message = error.to_string();
                    self.reporter.error(&format!("    ERROR {}", message,));
                    failures.push((name.clone(), message));
                    results.push((name.clone(), false));
                }
            }
        }

        assert!(
            results.len() == to_compile.len(),
            "results length must match to_compile length after batch",
        );

        Ok((results, failures))
    }

    fn compile_template(
        &self,
        name: &str,
        path: &Path,
        index: &TemplateIndex,
        vendor: &VendorIndex,
        output: &Path,
    ) -> Result<()> {
        assert!(!name.is_empty(), "template_name must not be empty",);

        assert!(path.is_file(), "template_path must be a file: {:?}", path,);

        assert!(output.exists(), "output directory must exist: {:?}", output,);

        let content = fs::read_to_string(path)?;
        let loader = FsTemplateLoader::new(index, vendor);

        let limits = ResolveLimits {
            max_include_depth: self.config.validation.max_include_depth,
            max_inheritance_depth: self.config.validation.max_inheritance_depth,
        };

        let parsed = compiler::parser::parse(&content)?;

        for diagnostic in &parsed.diagnostics {
            let label = match diagnostic.severity {
                compiler::error::Severity::Error => "ERROR",
                compiler::error::Severity::Warning => "WARN",
            };

            self.reporter
                .warn(&format!("    {} {}", label, diagnostic.message,));
        }

        let resolved = compiler::resolver::inheritance::resolve(
            parsed.nodes,
            name,
            Some(path.to_path_buf()),
            &loader,
            &limits,
        )?;

        let resolved = compiler::resolver::inclusion::resolve(resolved, name, &loader, &limits)?;

        let html = compiler::codegen::generate(&resolved);

        let target = output.join(name);

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = fs::File::create(&target)?;
        file.write_all(html.as_bytes())?;
        file.sync_all()?;

        assert!(
            target.exists(),
            "output file must exist after write: {:?}",
            target,
        );

        Ok(())
    }

    fn copy_remaining(
        &self,
        index: &TemplateIndex,
        output: &Path,
        compiled: &[(String, bool)],
    ) -> Result<u32> {
        let count = u32::try_from(index.templates.len()).expect("template count must fit in u32");

        assert!(
            count <= COPY_TEMPLATES_MAX,
            "template count exceeds maximum for copy",
        );

        assert!(
            output.exists(),
            "output directory must exist for copy_remaining: {:?}",
            output,
        );

        let names: FxHashSet<&str> = compiled
            .iter()
            .filter(|(_, success)| *success)
            .map(|(name, _)| name.as_str())
            .collect();

        let mut to_copy: Vec<(&String, &std::path::PathBuf)> = index
            .templates
            .iter()
            .filter(|(name, _)| !names.contains(name.as_str()))
            .filter(|(name, _)| !output.join(name).exists())
            .collect();

        to_copy.sort_by(|(a, _), (b, _)| a.cmp(b));

        let mut copied: u32 = 0;
        let mut iterations: u32 = 0;

        for (name, path) in to_copy {
            iterations += 1;

            assert!(
                iterations <= COPY_TEMPLATES_MAX,
                "copy_remaining exceeded {} iterations",
                COPY_TEMPLATES_MAX,
            );

            let target = output.join(name);

            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::copy(path, &target)?;
            self.reporter.info(&format!("  {}", name));
            copied += 1;
        }

        assert!(
            copied <= count,
            "copied count must not exceed template count",
        );

        Ok(copied)
    }
}
