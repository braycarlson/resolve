/// Filter applied to a variable: |default:"value"
#[derive(Debug, Clone, PartialEq)]
pub struct Filter {
    pub name: String,
    pub arguments: Vec<String>,
}

/// A condition and its body in an elif chain.
#[derive(Debug, Clone)]
pub struct ElifBranch {
    pub condition: String,
    pub body: Vec<AstNode>,
}

/// A name=value binding used by {% with %}, {% include ... with %},
/// and {% querystring %}.
#[derive(Debug, Clone)]
pub struct Binding {
    pub name: String,
    pub value: String,
}

/// Abstract Syntax Tree node
#[derive(Debug, Clone)]
pub enum AstNode {
    /// Plain text/HTML content
    Text(TextNode),

    /// Variable expression: {{ variable|filter }}
    Variable(Box<VariableNode>),

    /// Block definition: {% block name %}...{% endblock %}
    Block(Box<BlockNode>),

    /// Template inheritance: {% extends 'path' %}
    Extends(ExtendsNode),

    /// Template inclusion: {% include 'path' with ... %}
    Include(Box<IncludeNode>),

    /// Conditional: {% if %}...{% elif %}...{% else %}...{% endif %}
    If(Box<IfNode>),

    /// Loop: {% for %}...{% empty %}...{% endfor %}
    For(Box<ForNode>),

    /// Variable assignment: {% with %}...{% endwith %}
    With(Box<WithNode>),

    /// Library loading: {% load library %}
    Load(LoadNode),

    /// Static file: {% static 'path' %}
    Static(StaticNode),

    /// CSRF token: {% csrf_token %}
    Csrftoken(CsrftokenNode),

    /// Django comment block: {% comment %}...{% endcomment %}
    CommentBlock(CommentBlockNode),

    /// Autoescape block: {% autoescape off %}...{% endautoescape %}
    Autoescape(Box<AutoescapeNode>),

    /// Blocktranslate: {% blocktranslate %}...{% endblocktranslate %}
    Blocktranslate(Box<BlocktranslateNode>),

    /// Translation: {% trans 'message' %}
    Trans(TransNode),

    /// Language switch: {% language 'en' %}
    Language(Box<LanguageNode>),

    /// Verbatim block: {% verbatim %}...{% endverbatim %}
    Verbatim(VerbatimNode),

    /// Template tag output: {% templatetag 'openblock' %}
    TemplateTag(TemplateTagNode),

    /// Ifchanged conditional: {% ifchanged %}...{% endifchanged %}
    Ifchanged(Box<IfchangedNode>),

    /// Filter block: {% filter upper %}...{% endfilter %}
    FilterBlock(Box<FilterBlockNode>),

    /// Now tag: {% now "Y-m-d" %}
    Now(NowNode),

    /// Regroup tag: {% regroup list by field %}
    Regroup(Box<RegroupNode>),

    /// Cycle tag: {% cycle 'a' 'b' as var %}
    Cycle(Box<CycleNode>),

    /// Firstof tag: {% firstof var1 var2 %}
    Firstof(FirstofNode),

    /// Widthratio tag: {% widthratio val max 100 %}
    Widthratio(Box<WidthratioNode>),

    /// JSON script: {{ var|json_script 'id' }}
    JsonScript(Box<JsonScriptNode>),

    /// Capture as: {% url 'name' as var %}
    CaptureAs(Box<CaptureAsNode>),

    /// Cache block: {% cache timeout name %}...{% endcache %}
    Cache(Box<CacheNode>),

    /// Localize block: {% localize on/off %}...{% endlocalize %}
    Localize(Box<LocalizeNode>),

    /// Localtime block: {% localtime on/off %}...{% endlocaltime %}
    Localtime(Box<LocaltimeNode>),

    /// Spaceless block: {% spaceless %}...{% endspaceless %}
    Spaceless(SpacelessNode),

    /// Timezone block: {% timezone "tz" %}...{% endtimezone %}
    Timezone(Box<TimezoneNode>),

    /// UTC block: {% utc %}...{% endutc %}
    Utc(UtcNode),

    /// Debug tag: {% debug %}
    Debug(DebugNode),

    /// Get static prefix: {% get_static_prefix %}
    GetStaticPrefix(GetStaticPrefixNode),

    /// Get media prefix: {% get_media_prefix %}
    GetMediaPrefix(GetMediaPrefixNode),

    /// Reset cycle: {% resetcycle name %}
    Resetcycle(ResetcycleNode),

    /// Lorem tag: {% lorem count method random %}
    Lorem(Box<LoremNode>),

    /// Querystring tag: {% querystring key=val %}
    Querystring(Box<QuerystringNode>),

    /// Translate tag: {% translate "text" %}
    Translate(Box<TranslateNode>),

    /// Plural tag: {% plural %}
    Plural(PluralNode),
}

#[cfg(target_arch = "x86_64")]
const _: () = assert!(std::mem::size_of::<AstNode>() <= 56);

impl AstNode {
    /// Push all direct child node slices onto a traversal stack.
    /// Covers every variant that contains nested `Vec<AstNode>` fields.
    pub fn push_child_slices<'a>(&'a self, stack: &mut Vec<&'a [AstNode]>) {
        match self {
            AstNode::Block(block) => stack.push(&block.content),
            AstNode::If(if_node) => {
                stack.push(&if_node.true_branch);
                for elif in &if_node.elif_branches {
                    stack.push(&elif.body);
                }
                if let Some(else_branch) = &if_node.else_branch {
                    stack.push(else_branch);
                }
            }
            AstNode::For(for_node) => {
                stack.push(&for_node.body);
                if let Some(empty_branch) = &for_node.empty_branch {
                    stack.push(empty_branch);
                }
            }
            AstNode::With(with) => stack.push(&with.body),
            AstNode::Autoescape(autoescape) => stack.push(&autoescape.body),
            AstNode::Blocktranslate(blocktranslate) => stack.push(&blocktranslate.body),
            AstNode::Language(language) => stack.push(&language.body),
            AstNode::FilterBlock(filter_block) => stack.push(&filter_block.body),
            AstNode::Cache(cache) => stack.push(&cache.body),
            AstNode::Localize(localize) => stack.push(&localize.body),
            AstNode::Localtime(localtime) => stack.push(&localtime.body),
            AstNode::Spaceless(spaceless) => stack.push(&spaceless.body),
            AstNode::Timezone(timezone) => stack.push(&timezone.body),
            AstNode::Utc(utc) => stack.push(&utc.body),
            AstNode::Ifchanged(ifchanged) => {
                stack.push(&ifchanged.true_branch);
                if let Some(else_branch) = &ifchanged.else_branch {
                    stack.push(else_branch);
                }
            }
            AstNode::Text(_)
            | AstNode::Variable(_)
            | AstNode::Extends(_)
            | AstNode::Include(_)
            | AstNode::Load(_)
            | AstNode::Static(_)
            | AstNode::Csrftoken(_)
            | AstNode::CommentBlock(_)
            | AstNode::Trans(_)
            | AstNode::Verbatim(_)
            | AstNode::TemplateTag(_)
            | AstNode::Now(_)
            | AstNode::Regroup(_)
            | AstNode::Cycle(_)
            | AstNode::Firstof(_)
            | AstNode::Widthratio(_)
            | AstNode::JsonScript(_)
            | AstNode::CaptureAs(_)
            | AstNode::Debug(_)
            | AstNode::GetStaticPrefix(_)
            | AstNode::GetMediaPrefix(_)
            | AstNode::Resetcycle(_)
            | AstNode::Lorem(_)
            | AstNode::Querystring(_)
            | AstNode::Translate(_)
            | AstNode::Plural(_) => {}
        }
    }

    pub fn try_for_each_child_mut<E>(
        &mut self,
        mut callback: impl FnMut(&mut Vec<AstNode>) -> Result<(), E>,
    ) -> Result<(), E> {
        match self {
            AstNode::Block(block) => callback(&mut block.content)?,
            AstNode::If(if_node) => {
                callback(&mut if_node.true_branch)?;
                for elif in &mut if_node.elif_branches {
                    callback(&mut elif.body)?;
                }
                if let Some(ref mut branch) = if_node.else_branch {
                    callback(branch)?;
                }
            }
            AstNode::For(for_node) => {
                callback(&mut for_node.body)?;
                if let Some(ref mut branch) = for_node.empty_branch {
                    callback(branch)?;
                }
            }
            AstNode::With(with) => callback(&mut with.body)?,
            AstNode::Autoescape(autoescape) => callback(&mut autoescape.body)?,
            AstNode::Blocktranslate(blocktranslate) => callback(&mut blocktranslate.body)?,
            AstNode::Language(language) => callback(&mut language.body)?,
            AstNode::FilterBlock(filter_block) => callback(&mut filter_block.body)?,
            AstNode::Cache(cache) => callback(&mut cache.body)?,
            AstNode::Localize(localize) => callback(&mut localize.body)?,
            AstNode::Localtime(localtime) => callback(&mut localtime.body)?,
            AstNode::Spaceless(spaceless) => callback(&mut spaceless.body)?,
            AstNode::Timezone(timezone) => callback(&mut timezone.body)?,
            AstNode::Utc(utc) => callback(&mut utc.body)?,
            AstNode::Ifchanged(ifchanged) => {
                callback(&mut ifchanged.true_branch)?;
                if let Some(ref mut branch) = ifchanged.else_branch {
                    callback(branch)?;
                }
            }
            AstNode::Text(_)
            | AstNode::Variable(_)
            | AstNode::Extends(_)
            | AstNode::Include(_)
            | AstNode::Load(_)
            | AstNode::Static(_)
            | AstNode::Csrftoken(_)
            | AstNode::CommentBlock(_)
            | AstNode::Trans(_)
            | AstNode::Verbatim(_)
            | AstNode::TemplateTag(_)
            | AstNode::Now(_)
            | AstNode::Regroup(_)
            | AstNode::Cycle(_)
            | AstNode::Firstof(_)
            | AstNode::Widthratio(_)
            | AstNode::JsonScript(_)
            | AstNode::CaptureAs(_)
            | AstNode::Debug(_)
            | AstNode::GetStaticPrefix(_)
            | AstNode::GetMediaPrefix(_)
            | AstNode::Resetcycle(_)
            | AstNode::Lorem(_)
            | AstNode::Querystring(_)
            | AstNode::Translate(_)
            | AstNode::Plural(_) => {}
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TextNode {
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct VariableNode {
    pub raw: String,
    pub expression: String,
    pub filters: Vec<Filter>,
}

#[derive(Debug, Clone)]
pub struct BlockNode {
    pub raw: String,
    pub name: String,
    pub content: Vec<AstNode>,
    pub has_super_reference: bool,
}

#[derive(Debug, Clone)]
pub struct ExtendsNode {
    pub raw: String,
    pub parent_path: String,
}

#[derive(Debug, Clone)]
pub struct IncludeNode {
    pub raw: String,
    pub path: String,
    pub with_variables: Vec<Binding>,
    pub only: bool,
}

#[derive(Debug, Clone)]
pub struct IfNode {
    pub raw: String,
    pub condition: String,
    pub true_branch: Vec<AstNode>,
    pub elif_branches: Vec<ElifBranch>,
    pub else_branch: Option<Vec<AstNode>>,
}

#[derive(Debug, Clone)]
pub struct ForNode {
    pub raw: String,
    pub variable: String,
    pub iterable: String,
    pub body: Vec<AstNode>,
    pub empty_branch: Option<Vec<AstNode>>,
}

#[derive(Debug, Clone)]
pub struct WithNode {
    pub raw: String,
    pub bindings: Vec<Binding>,
    pub body: Vec<AstNode>,
}

#[derive(Debug, Clone)]
pub struct LoadNode {
    pub raw: String,
    pub libraries: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StaticNode {
    pub raw: String,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct CsrftokenNode {
    pub raw: String,
}

#[derive(Debug, Clone)]
pub struct CommentBlockNode {
    pub raw: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct AutoescapeNode {
    pub raw: String,
    pub value: String,
    pub body: Vec<AstNode>,
}

#[derive(Debug, Clone)]
pub struct BlocktranslateNode {
    pub raw: String,
    pub body: Vec<AstNode>,
    pub modifiers: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TransNode {
    pub raw: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct LanguageNode {
    pub raw: String,
    pub language: String,
    pub body: Vec<AstNode>,
}

#[derive(Debug, Clone)]
pub struct VerbatimNode {
    pub raw: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct TemplateTagNode {
    pub raw: String,
    pub format: String,
}

#[derive(Debug, Clone)]
pub struct IfchangedNode {
    pub raw: String,
    pub condition: Option<String>,
    pub true_branch: Vec<AstNode>,
    pub else_branch: Option<Vec<AstNode>>,
}

#[derive(Debug, Clone)]
pub struct FilterBlockNode {
    pub raw: String,
    pub filter: String,
    pub body: Vec<AstNode>,
}

#[derive(Debug, Clone)]
pub struct NowNode {
    pub raw: String,
    pub format: String,
}

#[derive(Debug, Clone)]
pub struct RegroupNode {
    pub raw: String,
    pub list: String,
    pub field: String,
    pub as_variable: String,
}

#[derive(Debug, Clone)]
pub struct CycleNode {
    pub raw: String,
    pub values: Vec<String>,
    pub as_variable: Option<String>,
    pub output: bool,
}

#[derive(Debug, Clone)]
pub struct FirstofNode {
    pub raw: String,
    pub variables: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WidthratioNode {
    pub raw: String,
    pub value: String,
    pub maximum: String,
    pub divisor: String,
}

#[derive(Debug, Clone)]
pub struct JsonScriptNode {
    pub raw: String,
    pub variable: String,
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct CaptureAsNode {
    pub raw: String,
    pub tag: String,
    pub content: String,
    pub variable_name: String,
}

#[derive(Debug, Clone)]
pub struct CacheNode {
    pub raw: String,
    pub timeout: String,
    pub name: String,
    pub body: Vec<AstNode>,
}

#[derive(Debug, Clone)]
pub struct LocalizeNode {
    pub raw: String,
    pub value: String,
    pub body: Vec<AstNode>,
}

#[derive(Debug, Clone)]
pub struct LocaltimeNode {
    pub raw: String,
    pub value: String,
    pub body: Vec<AstNode>,
}

#[derive(Debug, Clone)]
pub struct SpacelessNode {
    pub raw: String,
    pub body: Vec<AstNode>,
}

#[derive(Debug, Clone)]
pub struct TimezoneNode {
    pub raw: String,
    pub timezone: String,
    pub body: Vec<AstNode>,
}

#[derive(Debug, Clone)]
pub struct UtcNode {
    pub raw: String,
    pub body: Vec<AstNode>,
}

#[derive(Debug, Clone)]
pub struct DebugNode {
    pub raw: String,
}

#[derive(Debug, Clone)]
pub struct GetStaticPrefixNode {
    pub raw: String,
    pub variable_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GetMediaPrefixNode {
    pub raw: String,
    pub variable_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResetcycleNode {
    pub raw: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct LoremNode {
    pub raw: String,
    pub count: String,
    pub method: String,
    pub random: bool,
}

#[derive(Debug, Clone)]
pub struct QuerystringNode {
    pub raw: String,
    pub parameters: Vec<Binding>,
    pub variable_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TranslateNode {
    pub raw: String,
    pub message: String,
    pub variable_name: Option<String>,
    pub noop: bool,
}

#[derive(Debug, Clone)]
pub struct PluralNode {
    pub raw: String,
    pub content: String,
}
