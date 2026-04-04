pub mod alpine;
pub mod filters;

use anyhow::Result;

use crate::discovery::TemplateIndex;
use crate::loader::VendorIndex;
use crate::reporter::Reporter;
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
    pub warnings: Vec<String>,
    pub error_count: usize,
}

pub struct Validator {
    errors: Vec<ValidationError>,
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator {
    pub fn new() -> Self {
        Self {
            errors: Vec::with_capacity(32),
        }
    }

    pub fn validate(
        &mut self,
        index: &TemplateIndex,
        vendor: &VendorIndex,
        entries: &[String],
        reporter: &Reporter,
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

                self.validate_template(name, &parsed.nodes, index, vendor);
            }
        }

        let mut warnings = Vec::new();
        let mut errors: usize = 0;

        for error in &self.errors {
            match error.severity {
                Severity::Error => {
                    reporter.error(&error.to_string());
                    errors += 1;
                }
                Severity::Warning => {
                    warnings.push(error.to_string());
                }
            }
        }

        Ok(ValidationResult {
            warnings,
            error_count: errors,
        })
    }

    fn validate_template(
        &mut self,
        name: &str,
        nodes: &[AstNode],
        index: &TemplateIndex,
        vendor: &VendorIndex,
    ) {
        assert!(
            !name.is_empty(),
            "template_name must not be empty for validation",
        );

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

                if let AstNode::Include(include) = node
                    && crate::loader::find_in_vendor(&include.path, vendor).is_none()
                    && !index.templates.contains_key(&include.path)
                {
                    self.errors.push(ValidationError {
                        template: name.to_string(),
                        message: format!("Include template not found: {}", include.path),
                        severity: Severity::Error,
                    });
                }

                node.push_child_slices(&mut stack);
            }
        }
    }
}
