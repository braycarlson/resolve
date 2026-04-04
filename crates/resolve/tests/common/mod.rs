#![allow(dead_code)]

use std::fs;

use tempfile::TempDir;

use compiler::ast::AstNode;
use compiler::codegen::generate;
use compiler::error::ParseError;
use compiler::resolver::ResolveLimits;
use resolve::config::{
    CompilerConfig, Config, EntryTemplatesConfig, IncrementalConfig, PathsConfig, ValidationConfig,
    VendorConfig,
};
use resolve::loader::{FsTemplateLoader, VendorIndex};


pub fn create_test_config(temp_dir: &TempDir) -> Config {
    Config {
        compiler: CompilerConfig {
            output_directory: temp_dir.path().join("output"),
            cache_directory: temp_dir.path().join("cache"),
        },
        paths: PathsConfig {
            primary_templates: vec![temp_dir.path().to_path_buf()],
            app_templates: vec![],
        },
        vendor: VendorConfig {
            auto_detect: false,
            vendor_directory: temp_dir.path().join("vendor"),
            virtual_environment_path: None,
        },
        entry_templates: EntryTemplatesConfig {
            auto_discover: false,
            explicit: vec![],
        },
        validation: ValidationConfig {
            strict: false,
            warn_undefined_vars: false,
            max_include_depth: 20,
            max_inheritance_depth: 10,
            skip_url_validation: true,
        },
        incremental: IncrementalConfig {
            enabled: false,
            track_file_hashes: false,
        },
    }
}

pub fn write_template(temp_dir: &TempDir, name: &str, content: &str) {
    let path = temp_dir.path().join(name);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    fs::write(path, content).unwrap();
}

pub fn parse(content: &str) -> Result<Vec<AstNode>, ParseError> {
    compiler::parser::parse(content).map(|output| output.nodes)
}

pub fn compile_simple(content: &str) -> String {
    let nodes = parse(content).unwrap();
    generate(&nodes)
}

pub fn compile_template(temp_dir: &TempDir, name: &str) -> String {
    let config = create_test_config(temp_dir);
    let discovery = resolve::discovery::TemplateDiscovery::new(&config);
    let index = discovery.scan().unwrap();

    let vendor_index = VendorIndex::build(temp_dir.path().join("vendor").as_path());
    let loader = FsTemplateLoader::new(&index, &vendor_index);

    let limits = ResolveLimits {
        max_include_depth: config.validation.max_include_depth,
        max_inheritance_depth: config.validation.max_inheritance_depth,
    };

    let template_path = temp_dir.path().join(name);
    let content = fs::read_to_string(&template_path).unwrap();

    let nodes = parse(&content).unwrap();

    let resolved = compiler::resolver::inheritance::resolve(
        nodes,
        name,
        Some(template_path.to_path_buf()),
        &loader,
        &limits,
    )
    .unwrap_or_else(|_| parse(&content).unwrap());

    let resolved = compiler::resolver::inclusion::resolve(
        resolved,
        name,
        &loader,
        &limits,
    )
    .unwrap_or_else(|_| parse(&content).unwrap());

    generate(&resolved)
}

#[derive(Clone, Copy)]
pub enum NodeType {
    Block,
    Csrftoken,
    Extends,
    For,
    If,
    Include,
    Load,
}

pub fn count_nodes_of_type(nodes: &[AstNode], node_type: NodeType) -> usize {
    let mut count: usize = 0;

    for node in nodes {
        match node_type {
            NodeType::Block => {
                if matches!(node, AstNode::Block(_)) {
                    count += 1;
                }
            }
            NodeType::Csrftoken => {
                if matches!(node, AstNode::Csrftoken(_)) {
                    count += 1;
                }
            }
            NodeType::Extends => {
                if matches!(node, AstNode::Extends(_)) {
                    count += 1;
                }
            }
            NodeType::For => {
                if matches!(node, AstNode::For(_)) {
                    count += 1;
                }
            }
            NodeType::If => {
                if matches!(node, AstNode::If(_)) {
                    count += 1;
                }
            }
            NodeType::Include => {
                if matches!(node, AstNode::Include(_)) {
                    count += 1;
                }
            }
            NodeType::Load => {
                if matches!(node, AstNode::Load(_)) {
                    count += 1;
                }
            }
        }

        match node {
            AstNode::If(if_node) => {
                count += count_nodes_of_type(&if_node.true_branch, node_type);

                for elif in &if_node.elif_branches {
                    count += count_nodes_of_type(&elif.body, node_type);
                }

                if let Some(else_branch) = &if_node.else_branch {
                    count += count_nodes_of_type(else_branch, node_type);
                }
            }
            AstNode::For(for_node) => {
                count += count_nodes_of_type(&for_node.body, node_type);

                if let Some(empty) = &for_node.empty_branch {
                    count += count_nodes_of_type(empty, node_type);
                }
            }
            AstNode::Block(block_node) => {
                count += count_nodes_of_type(&block_node.content, node_type);
            }
            AstNode::With(with_node) => {
                count += count_nodes_of_type(&with_node.body, node_type);
            }
            AstNode::Autoescape(auto_node) => {
                count += count_nodes_of_type(&auto_node.body, node_type);
            }
            AstNode::Language(lang_node) => {
                count += count_nodes_of_type(&lang_node.body, node_type);
            }
            AstNode::FilterBlock(filter_node) => {
                count += count_nodes_of_type(&filter_node.body, node_type);
            }
            _ => {}
        }
    }

    count
}
