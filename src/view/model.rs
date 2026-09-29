#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub enum DefinitionsMode {
    All,
    Referenced,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ViewOptions {
    pub verbose: bool,
    pub max_depth: Option<usize>,
    pub definitions: DefinitionsMode,
    pub expand_refs: bool,
    pub annotations: bool,
}

impl Default for ViewOptions {
    fn default() -> Self {
        Self {
            verbose: false,
            max_depth: None,
            definitions: DefinitionsMode::All,
            expand_refs: false,
            annotations: true,
        }
    }
}

pub struct ViewDocument {
    pub roots: Vec<ViewNode>,
}

pub struct ViewNode {
    pub role: ViewNodeRole,
    pub key: Option<String>,
    pub value: String,
    pub details: Vec<ViewDetail>,
    pub children: Vec<ViewNode>,
}

pub enum ViewNodeRole {
    Schema(SchemaNodeRole),
    Constraint,
    Section,
}

pub enum SchemaNodeRole {
    Instance,
    Definition,
    PatternProperty,
    PropertyName,
    Contains,
    ContentSchema,
    UnevaluatedProperties,
    UnevaluatedItems,
    ReferenceApplicator,
    LogicalBranch,
    DependentSchema,
    Condition,
    ThenBranch,
    ElseBranch,
}

pub struct ViewDetail {
    pub kind: ViewDetailKind,
    pub text: String,
}

pub enum ViewDetailKind {
    Constraint,
    Annotation,
    Metadata,
    Marker,
}

impl ViewDetail {
    pub fn constraint(text: impl Into<String>) -> Self {
        Self {
            kind: ViewDetailKind::Constraint,
            text: text.into(),
        }
    }

    pub fn annotation(text: impl Into<String>) -> Self {
        Self {
            kind: ViewDetailKind::Annotation,
            text: text.into(),
        }
    }

    pub fn metadata(text: impl Into<String>) -> Self {
        Self {
            kind: ViewDetailKind::Metadata,
            text: text.into(),
        }
    }

    pub fn marker(text: impl Into<String>) -> Self {
        Self {
            kind: ViewDetailKind::Marker,
            text: text.into(),
        }
    }
}
