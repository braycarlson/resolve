use rustc_hash::{FxHashMap, FxHashSet};

use crate::ast::*;
use crate::error::CompileError;
use crate::parser;
use crate::resolver::inheritance;
use crate::resolver::{ResolveLimits, TemplateLoader};

const RESOLVE_NODES_MAX: u32 = 500_000;
const AST_DEPTH_MAX: u32 = 128;

struct IncludeState {
    expanding: FxHashMap<String, u32>,
    recursive: FxHashSet<String>,
}

impl IncludeState {
    fn new() -> Self {
        Self {
            expanding: FxHashMap::default(),
            recursive: FxHashSet::default(),
        }
    }

    fn begin_expand(&mut self, path: &str) {
        assert!(!path.is_empty(), "path must not be empty for begin_expand",);

        *self.expanding.entry(path.to_string()).or_insert(0) += 1;
    }

    fn end_expand(&mut self, path: &str) {
        assert!(!path.is_empty(), "path must not be empty for end_expand",);

        if let Some(count) = self.expanding.get_mut(path) {
            *count -= 1;

            if *count == 0 {
                self.expanding.remove(path);
            }
        }
    }

    fn is_expanding(&self, path: &str) -> bool {
        assert!(!path.is_empty(), "path must not be empty for is_expanding",);

        self.expanding.get(path).is_some_and(|&count| count > 0)
    }

    fn mark_recursive(&mut self, path: &str) {
        assert!(
            !path.is_empty(),
            "path must not be empty for mark_recursive",
        );

        self.recursive.insert(path.to_string());
    }
}

#[cold]
#[inline(never)]
fn error_limit(template: &str) -> CompileError {
    CompileError::NodeLimitExceeded {
        template: template.to_string(),
        max: RESOLVE_NODES_MAX,
    }
}

#[cold]
#[inline(never)]
fn error_parsed(path: &str, count: usize) -> CompileError {
    CompileError::ParsedNodeLimitExceeded {
        path: path.to_string(),
        count,
        max: RESOLVE_NODES_MAX,
    }
}

#[cold]
#[inline(never)]
fn error_depth() -> CompileError {
    CompileError::AstDepthExceeded {
        max_depth: AST_DEPTH_MAX,
    }
}

pub fn resolve<L: TemplateLoader>(
    ast: Vec<AstNode>,
    template: &str,
    loader: &L,
    limits: &ResolveLimits,
) -> Result<Vec<AstNode>, CompileError> {
    assert!(!template.is_empty(), "template_name must not be empty",);

    assert!(
        limits.max_include_depth > 0,
        "max_include_depth must be greater than zero",
    );

    let mut current = ast;
    let mut cache: FxHashMap<String, Vec<AstNode>> = FxHashMap::default();
    let mut state = IncludeState::new();

    for _ in 0..limits.max_include_depth {
        let mut expanded = false;

        let count = u32::try_from(current.len()).expect("current node count must fit in u32");

        if count > RESOLVE_NODES_MAX {
            return Err(error_limit(template));
        }

        current = expand_pass(
            current,
            loader,
            &mut cache,
            &mut expanded,
            0,
            limits,
            &mut state,
        )?;

        if !expanded {
            break;
        }
    }

    Ok(current)
}

fn expand_pass<L: TemplateLoader>(
    nodes: Vec<AstNode>,
    loader: &L,
    cache: &mut FxHashMap<String, Vec<AstNode>>,
    expanded: &mut bool,
    depth: u32,
    limits: &ResolveLimits,
    state: &mut IncludeState,
) -> Result<Vec<AstNode>, CompileError> {
    if depth > AST_DEPTH_MAX {
        return Err(error_depth());
    }

    assert!(
        nodes.len() <= RESOLVE_NODES_MAX as usize,
        "expand_pass input exceeds maximum node count",
    );

    let mut result = Vec::with_capacity(nodes.len());

    for node in nodes {
        match node {
            AstNode::Include(ref include) => {
                match expand_include(include, loader, cache, limits, state)? {
                    IncludeResult::Preserved(preserved) => {
                        result.push(preserved);
                    }

                    IncludeResult::Inlined(inlined) => {
                        *expanded = true;
                        state.begin_expand(&include.path);

                        let processed = expand_pass(
                            inlined,
                            loader,
                            cache,
                            expanded,
                            depth + 1,
                            limits,
                            state,
                        )?;

                        state.end_expand(&include.path);

                        result.extend(processed);
                    }
                }
            }

            _ => {
                let processed =
                    expand_children(node, loader, cache, expanded, depth, limits, state)?;

                result.push(processed);
            }
        }
    }

    Ok(result)
}

enum IncludeResult {
    Preserved(AstNode),
    Inlined(Vec<AstNode>),
}

fn preserve_include(include: &IncludeNode) -> IncludeResult {
    assert!(
        !include.path.is_empty(),
        "include path must not be empty for preserve",
    );

    IncludeResult::Preserved(AstNode::Include(Box::new(include.clone())))
}

fn expand_include<L: TemplateLoader>(
    include: &IncludeNode,
    loader: &L,
    cache: &mut FxHashMap<String, Vec<AstNode>>,
    limits: &ResolveLimits,
    state: &mut IncludeState,
) -> Result<IncludeResult, CompileError> {
    assert!(!include.path.is_empty(), "include path must not be empty",);

    if include.only {
        return Ok(preserve_include(include));
    }

    if state.recursive.contains(&include.path) {
        return Ok(preserve_include(include));
    }

    if state.is_expanding(&include.path) {
        state.mark_recursive(&include.path);
        return Ok(preserve_include(include));
    }

    let included = load_cached(&include.path, loader, cache, limits)?;

    let Some(included) = included else {
        return Ok(preserve_include(include));
    };

    if include.with_variables.is_empty() {
        return Ok(IncludeResult::Inlined(included));
    }

    let raw = include
        .with_variables
        .iter()
        .map(|binding| format!("{}={}", binding.name, binding.value))
        .collect::<Vec<_>>()
        .join(" ");

    let node = AstNode::With(Box::new(WithNode {
        raw: format!("{{% with {raw} %}}"),
        bindings: include.with_variables.clone(),
        body: included,
    }));

    Ok(IncludeResult::Inlined(vec![node]))
}

fn load_cached<L: TemplateLoader>(
    path: &str,
    loader: &L,
    cache: &mut FxHashMap<String, Vec<AstNode>>,
    limits: &ResolveLimits,
) -> Result<Option<Vec<AstNode>>, CompileError> {
    assert!(!path.is_empty(), "include path must not be empty",);

    if let Some(cached) = cache.get(path) {
        return Ok(Some(cached.clone()));
    }

    let Some((resolved, content)) = loader.load(path)? else {
        return Ok(None);
    };

    let mut ast = parser::parse(&content).map_err(CompileError::from)?.nodes;

    assert!(
        u32::try_from(ast.len()).is_ok(),
        "parsed node count exceeds u32 range",
    );

    if ast.len() > RESOLVE_NODES_MAX as usize {
        return Err(error_parsed(path, ast.len()));
    }

    let has_extends = ast.iter().any(|node| matches!(node, AstNode::Extends(_)));

    if has_extends {
        ast = inheritance::resolve(ast, path, Some(resolved), loader, limits)?;
    }

    cache.insert(path.to_string(), ast.clone());

    Ok(Some(ast))
}

fn expand_children<L: TemplateLoader>(
    mut node: AstNode,
    loader: &L,
    cache: &mut FxHashMap<String, Vec<AstNode>>,
    expanded: &mut bool,
    depth: u32,
    limits: &ResolveLimits,
    state: &mut IncludeState,
) -> Result<AstNode, CompileError> {
    assert!(
        depth <= AST_DEPTH_MAX,
        "expand_children depth exceeds AST_DEPTH_MAX",
    );

    assert!(
        limits.max_include_depth > 0,
        "max_include_depth must be greater than zero in expand_children",
    );

    node.try_for_each_child_mut(|children| {
        let taken = std::mem::take(children);

        *children = expand_pass(taken, loader, cache, expanded, depth + 1, limits, state)?;

        Ok::<(), CompileError>(())
    })?;

    Ok(node)
}
