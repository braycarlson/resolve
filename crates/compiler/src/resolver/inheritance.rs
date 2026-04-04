use std::path::PathBuf;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::ast::*;
use crate::error::CompileError;
use crate::parser;
use crate::resolver::{ResolveLimits, TemplateLoader};


const BLOCK_NESTING_DEPTH_MAX: u32 = 32;
const EXTRACT_ITERATIONS_MAX: u32 = 500_000;
const REPLACE_NODES_MAX: u32 = 500_000;
const MERGE_ENTRIES_MAX: u32 = 10_000;
const SPLICE_NODES_MAX: u32 = 100_000;
const AST_DEPTH_MAX: u32 = 128;
const CHAIN_MERGE_ITERATIONS_MAX: u32 = 128;
const INJECT_ITERATIONS_MAX: u32 = 10_000;
const COLLECT_LOADS_ITERATIONS_MAX: u32 = 500_000;

#[cold]
#[inline(never)]
fn error_circular(name: &str) -> CompileError {
    CompileError::CircularInheritance {
        template: name.to_string(),
    }
}

#[cold]
#[inline(never)]
fn error_parent(path: &str) -> CompileError {
    CompileError::ParentNotFound {
        path: path.to_string(),
    }
}

#[cold]
#[inline(never)]
fn error_empty(template: &str) -> CompileError {
    CompileError::EmptyInheritanceChain {
        template: template.to_string(),
    }
}

#[cold]
#[inline(never)]
fn error_chain_depth(limit: u32, name: &str) -> CompileError {
    CompileError::InheritanceDepthExceeded {
        max_depth: limit,
        template: name.to_string(),
    }
}

#[cold]
#[inline(never)]
fn error_ast_depth() -> CompileError {
    CompileError::AstDepthExceeded {
        max_depth: AST_DEPTH_MAX,
    }
}

pub fn resolve<L: TemplateLoader>(
    ast: Vec<AstNode>,
    template: &str,
    resolved: Option<PathBuf>,
    loader: &L,
    limits: &ResolveLimits,
) -> Result<Vec<AstNode>, CompileError> {
    assert!(
        !template.is_empty(),
        "template_name must not be empty",
    );

    assert!(
        limits.max_inheritance_depth > 0,
        "max_inheritance_depth must be greater than zero",
    );

    resolve_chain(ast, template, resolved, loader, limits)
}

fn resolve_chain<L: TemplateLoader>(
    ast: Vec<AstNode>,
    template: &str,
    resolved: Option<PathBuf>,
    loader: &L,
    limits: &ResolveLimits,
) -> Result<Vec<AstNode>, CompileError> {
    let limit = limits.max_inheritance_depth;

    let mut visited: FxHashSet<PathBuf> =
        FxHashSet::with_capacity_and_hasher(
            limit as usize,
            Default::default(),
        );

    let mut chain: Vec<(String, Vec<AstNode>)> =
        Vec::with_capacity(limit as usize);

    let mut nodes = ast;
    let mut name = template.to_string();
    let mut path: Option<PathBuf> = resolved;

    for _ in 0..=limit {
        let length = u32::try_from(chain.len())
            .expect("chain length must fit in u32");

        if length > limit {
            return Err(error_chain_depth(limit, &name));
        }

        if let Some(ref p) = path {
            if visited.contains(p) {
                return Err(error_circular(&name));
            }

            visited.insert(p.clone());
        }

        let extends = find_extends(&nodes);

        match extends {
            Some(extends) => {
                let parent = extends.parent_path.clone();

                chain.push((name.clone(), nodes));

                let (resolved, content) = load_parent(
                    &parent,
                    path.as_deref(),
                    loader,
                )?;

                nodes = parser::parse(&content)
                    .map_err(CompileError::from)?
                    .nodes;

                name = parent;
                path = Some(resolved);
            }

            None => {
                chain.push((name, nodes));
                break;
            }
        }
    }

    if chain.is_empty() {
        return Err(error_empty(template));
    }

    let result = merge_chain(chain)?;

    Ok(result)
}

fn find_extends(ast: &[AstNode]) -> Option<&ExtendsNode> {
    assert!(
        u32::try_from(ast.len()).is_ok(),
        "ast length must fit in u32 range",
    );

    ast.iter().find_map(|node| {
        if let AstNode::Extends(extends) = node {
            Some(extends)
        } else {
            None
        }
    })
}

fn load_parent<L: TemplateLoader>(
    parent: &str,
    exclude: Option<&std::path::Path>,
    loader: &L,
) -> Result<(PathBuf, String), CompileError> {
    assert!(
        !parent.is_empty(),
        "parent_path must not be empty",
    );

    loader
        .load_excluding(parent, exclude)?
        .ok_or_else(|| error_parent(parent))
}

fn merge_chain(
    mut chain: Vec<(String, Vec<AstNode>)>,
) -> Result<Vec<AstNode>, CompileError> {
    assert!(
        !chain.is_empty(),
        "inheritance chain must have at least one template",
    );

    let length = u32::try_from(chain.len())
        .expect("chain length must fit in u32");

    assert!(
        length <= BLOCK_NESTING_DEPTH_MAX,
        "inheritance chain length exceeds maximum",
    );

    let loads = collect_loads(&chain);

    let (_, root) = chain.pop().expect(
        "chain must be non-empty (asserted above)",
    );

    let mut resolved = root;
    let mut iterations: u32 = 0;

    while let Some((_, child)) = chain.pop() {
        iterations += 1;

        assert!(
            iterations <= CHAIN_MERGE_ITERATIONS_MAX,
            "merge_chain exceeded {} iterations",
            CHAIN_MERGE_ITERATIONS_MAX,
        );

        let child = extract_blocks(&child);
        let parent = extract_blocks(&resolved);
        let merged = merge_blocks(&parent, &child);
        resolved = replace_blocks(resolved, &merged)?;
    }

    inject_loads(&mut resolved, loads);

    Ok(resolved)
}

fn collect_loads(
    chain: &[(String, Vec<AstNode>)],
) -> Vec<AstNode> {
    assert!(
        u32::try_from(chain.len()).is_ok(),
        "chain length must fit in u32 for collect_loads",
    );

    let mut loads: Vec<AstNode> = Vec::new();
    let mut seen: FxHashSet<Vec<String>> = FxHashSet::default();
    let mut iterations: u32 = 0;

    for (_name, ast) in chain {
        for node in ast {
            iterations += 1;

            assert!(
                iterations <= COLLECT_LOADS_ITERATIONS_MAX,
                "collect_loads exceeded {} iterations",
                COLLECT_LOADS_ITERATIONS_MAX,
            );

            if let AstNode::Load(load) = node {
                if seen.insert(load.libraries.clone()) {
                    loads.push(node.clone());
                }
            }
        }
    }

    loads
}

fn inject_loads(
    resolved: &mut Vec<AstNode>,
    loads: Vec<AstNode>,
) {
    let count = u32::try_from(loads.len())
        .expect("loads length must fit in u32");

    assert!(
        count <= MERGE_ENTRIES_MAX,
        "loads count exceeds maximum",
    );

    let existing: FxHashSet<Vec<String>> = resolved
        .iter()
        .filter_map(|node| {
            if let AstNode::Load(load) = node {
                Some(load.libraries.clone())
            } else {
                None
            }
        })
        .collect();

    let mut inject: Vec<AstNode> = Vec::new();
    let mut iterations: u32 = 0;

    for load in loads {
        iterations += 1;

        assert!(
            iterations <= INJECT_ITERATIONS_MAX,
            "inject_loads filter exceeded {} iterations",
            INJECT_ITERATIONS_MAX,
        );

        if let AstNode::Load(ref node) = load {
            if !existing.contains(&node.libraries) {
                inject.push(load);
            }
        }
    }

    if inject.is_empty() {
        return;
    }

    let offset = resolved
        .iter()
        .position(|node| matches!(node, AstNode::Load(_)))
        .or_else(|| {
            resolved
                .iter()
                .position(|node| !matches!(node, AstNode::Text(_)))
        })
        .unwrap_or(0);

    assert!(
        offset <= resolved.len(),
        "offset must not exceed resolved length",
    );

    let mut count: u32 = 0;

    for (index, load) in inject.into_iter().enumerate() {
        count += 1;

        assert!(
            count <= INJECT_ITERATIONS_MAX,
            "inject_loads insert exceeded {} iterations",
            INJECT_ITERATIONS_MAX,
        );

        resolved.insert(offset + index, load);
    }
}

fn extract_blocks<'a>(
    nodes: &'a [AstNode],
) -> FxHashMap<&'a str, &'a [AstNode]> {
    assert!(
        nodes.len() <= REPLACE_NODES_MAX as usize,
        "extract_blocks input exceeds maximum node count",
    );

    let mut blocks: FxHashMap<&'a str, &'a [AstNode]> =
        FxHashMap::with_capacity_and_hasher(8, Default::default());

    let mut stack: Vec<&'a [AstNode]> = Vec::with_capacity(32);

    stack.push(nodes);

    let mut iterations: u32 = 0;

    while let Some(current) = stack.pop() {
        for node in current {
            iterations += 1;

            assert!(
                iterations <= EXTRACT_ITERATIONS_MAX,
                "extract_blocks exceeded maximum iterations",
            );

            if let AstNode::Block(block) = node {
                blocks.insert(&block.name, &block.content);
            }

            node.push_child_slices(&mut stack);
        }
    }

    blocks
}

fn merge_blocks(
    parent: &FxHashMap<&str, &[AstNode]>,
    child: &FxHashMap<&str, &[AstNode]>,
) -> FxHashMap<String, Vec<AstNode>> {
    let count = u32::try_from(parent.len())
        .expect("parent length must fit in u32");

    assert!(
        count <= MERGE_ENTRIES_MAX,
        "parent count exceeds maximum",
    );

    let count = u32::try_from(child.len())
        .expect("child length must fit in u32");

    assert!(
        count <= MERGE_ENTRIES_MAX,
        "child count exceeds maximum",
    );

    let mut merged: FxHashMap<String, Vec<AstNode>> =
        FxHashMap::with_capacity_and_hasher(
            parent.len() + child.len(),
            Default::default(),
        );

    let mut iterations: u32 = 0;

    for (&name, &content) in parent {
        iterations += 1;

        assert!(
            iterations <= MERGE_ENTRIES_MAX,
            "merge_blocks parent iteration exceeded maximum",
        );

        merged.insert(name.to_string(), content.to_vec());
    }

    let mut iterations: u32 = 0;

    for (&name, &content) in child {
        iterations += 1;

        assert!(
            iterations <= MERGE_ENTRIES_MAX,
            "merge_blocks child iteration exceeded maximum",
        );

        let has_super = content.iter().any(|node| {
            matches!(
                node,
                AstNode::Variable(variable)
                    if variable.expression.trim() == "block.super"
            )
        });

        if has_super {
            let spliced = splice_super(
                name,
                content,
                &merged,
            );

            merged.insert(name.to_string(), spliced);
        } else {
            merged.insert(name.to_string(), content.to_vec());
        }
    }

    merged
}

fn splice_super(
    name: &str,
    content: &[AstNode],
    merged: &FxHashMap<String, Vec<AstNode>>,
) -> Vec<AstNode> {
    assert!(
        !name.is_empty(),
        "block name must not be empty for splice_super",
    );

    let Some(parent) = merged.get(name) else {
        return content.to_vec();
    };

    let total = u32::try_from(
        content.len() + parent.len()
    ).expect("splice total must fit in u32");

    assert!(
        total <= SPLICE_NODES_MAX,
        "block.super splice exceeds maximum node count",
    );

    let mut spliced = Vec::with_capacity(total as usize);
    let mut iterations: u32 = 0;

    for node in content {
        iterations += 1;

        assert!(
            iterations <= SPLICE_NODES_MAX,
            "splice_super exceeded {} iterations",
            SPLICE_NODES_MAX,
        );

        if let AstNode::Variable(variable) = node {
            if variable.expression.trim() == "block.super" {
                spliced.extend(parent.iter().cloned());
                continue;
            }
        }

        spliced.push(node.clone());
    }

    spliced
}

fn replace_blocks(
    nodes: Vec<AstNode>,
    blocks: &FxHashMap<String, Vec<AstNode>>,
) -> Result<Vec<AstNode>, CompileError> {
    assert!(
        nodes.len() <= REPLACE_NODES_MAX as usize,
        "replace_blocks input exceeds maximum node count",
    );

    let count = u32::try_from(blocks.len())
        .expect("blocks length must fit in u32");

    assert!(
        count <= MERGE_ENTRIES_MAX,
        "replace_blocks block count exceeds maximum",
    );

    replace_bounded(nodes, blocks, 0)
}

fn replace_bounded(
    nodes: Vec<AstNode>,
    blocks: &FxHashMap<String, Vec<AstNode>>,
    depth: u32,
) -> Result<Vec<AstNode>, CompileError> {
    if depth > AST_DEPTH_MAX {
        return Err(error_ast_depth());
    }

    assert!(
        nodes.len() <= REPLACE_NODES_MAX as usize,
        "replace_bounded input exceeds maximum node count",
    );

    let mut result = Vec::with_capacity(nodes.len());

    for node in nodes {
        match node {
            AstNode::Block(ref block) => {
                let content = blocks
                    .get(&block.name)
                    .cloned()
                    .unwrap_or_else(|| block.content.clone());

                let processed =
                    replace_bounded(content, blocks, depth + 1)?;

                result.push(AstNode::Block(Box::new(BlockNode {
                    raw: block.raw.clone(),
                    name: block.name.clone(),
                    content: processed,
                    has_super_reference: block.has_super_reference,
                })));
            }

            AstNode::Extends(_) => {}

            _ => {
                let processed =
                    replace_children(node, blocks, depth)?;

                result.push(processed);
            }
        }
    }

    Ok(result)
}

fn replace_children(
    mut node: AstNode,
    blocks: &FxHashMap<String, Vec<AstNode>>,
    depth: u32,
) -> Result<AstNode, CompileError> {
    assert!(
        depth <= AST_DEPTH_MAX,
        "replace_children depth exceeds AST_DEPTH_MAX",
    );

    let count = u32::try_from(blocks.len())
        .expect("blocks length must fit in u32");

    assert!(
        count <= MERGE_ENTRIES_MAX,
        "replace_children block count exceeds maximum",
    );

    node.try_for_each_child_mut(|children| {
        let taken = std::mem::take(children);
        *children = replace_bounded(taken, blocks, depth + 1)?;
        Ok::<(), CompileError>(())
    })?;

    Ok(node)
}
