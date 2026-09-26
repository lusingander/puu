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

    pub fn marker(text: impl Into<String>) -> Self {
        Self {
            kind: ViewDetailKind::Marker,
            text: text.into(),
        }
    }
}
