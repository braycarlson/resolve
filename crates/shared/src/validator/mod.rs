pub mod alpine;
pub mod filters;

use std::path::Path;

use anyhow::Result;

use crate::discovery::TemplateIndex;
use crate::loader::VendorIndex;
use compiler::ast::*;
use compiler::error::Severity;

const VALIDATION_NODES_MAX: u32 = 500_000;

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub template: String,
    pub message: String,
    pub severity: Severity,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self.severity {
            Severity::Error => "ERROR",
            Severity::Warning => "WARNING",
        };

        write!(formatter, "[{}] {}: {}", label, self.template, self.message,)
    }
}

pub struct ValidationResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

pub struct Validator {
    issues: Vec<ValidationError>,
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator {
    pub fn new() -> Self {
        Self {
            issues: Vec::with_capacity(32),
        }
    }

    pub fn validate(
        &mut self,
        index: &TemplateIndex,
        vendor: &VendorIndex,
        entries: &[String],
        vendor_path: &Path,
    ) -> Result<ValidationResult> {
        let count = u32::try_from(entries.len()).expect("entry_templates length must fit in u32");

        assert!(
            count <= VALIDATION_NODES_MAX,
            "entry template count exceeds validation maximum",
        );

        for name in entries {
            if let Some(path) = index.templates.get(name) {
                let content = std::fs::read_to_string(path)?;
                let parsed = compiler::parser::parse(&content)?;

                let is_vendor = path.starts_with(vendor_path);

                self.validate_template(name, &parsed.nodes, index, vendor, is_vendor);
            }
        }

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        for issue in &self.issues {
            match issue.severity {
                Severity::Error => errors.push(issue.to_string()),
                Severity::Warning => warnings.push(issue.to_string()),
            }
        }

        Ok(ValidationResult { errors, warnings })
    }

    fn validate_template(
        &mut self,
        name: &str,
        nodes: &[AstNode],
        index: &TemplateIndex,
        vendor: &VendorIndex,
        is_vendor: bool,
    ) {
        assert!(
            !name.is_empty(),
            "template_name must not be empty for validation",
        );

        let severity = if is_vendor {
            Severity::Warning
        } else {
            Severity::Error
        };

        let mut stack: Vec<&[AstNode]> = vec![nodes];
        let mut iterations: u32 = 0;

        while let Some(nodes) = stack.pop() {
            for node in nodes {
                iterations += 1;

                assert!(
                    iterations <= VALIDATION_NODES_MAX,
                    "validate_template exceeded {} iterations for {}",
                    VALIDATION_NODES_MAX,
                    name,
                );

                if let AstNode::Extends(extends) = node
                    && crate::loader::find_in_vendor(&extends.parent_path, vendor).is_none()
                    && !index.templates.contains_key(&extends.parent_path)
                {
                    self.issues.push(ValidationError {
                        template: name.to_string(),
                        message: format!("Parent template not found: {}", extends.parent_path),
                        severity,
                    });
                }

                if let AstNode::Include(include) = node
                    && crate::loader::find_in_vendor(&include.path, vendor).is_none()
                    && !index.templates.contains_key(&include.path)
                {
                    self.issues.push(ValidationError {
                        template: name.to_string(),
                        message: format!("Include template not found: {}", include.path),
                        severity,
                    });
                }

                node.push_child_slices(&mut stack);
            }
        }
    }
}
