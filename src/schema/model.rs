use serde_json::{Number, Value};

use crate::dialect::Dialect;

pub struct Schema {
    pub location: String,
    pub identity: SchemaIdentity,
    pub dialect: Dialect,
    pub overridden_dialect: Option<String>,
    pub kind: SchemaKind,
    pub reference: Option<Reference>,
    pub recursive_reference: Option<Reference>,
    pub dynamic_reference: Option<Reference>,
    pub definitions: Vec<(String, Schema)>,
    pub properties: Vec<(String, Schema)>,
    pub required_names: Vec<String>,
    pub object_constraints: ObjectConstraints,
    pub array_constraints: ArrayConstraints,
    pub logical_applicators: LogicalApplicators,
    pub conditional_applicators: ConditionalApplicators,
    pub string_constraints: StringConstraints,
    pub number_constraints: NumberConstraints,
    pub value_constraints: ValueConstraints,
    pub annotations: Annotations,
    pub ignored_keywords: Vec<(String, Value)>,
    pub uninterpreted_keywords: Vec<(String, Value)>,
}

#[derive(Default)]
pub struct SchemaIdentity {
    pub identifier: Option<String>,
    pub anchor: Option<String>,
    pub recursive_anchor: Option<bool>,
    pub dynamic_anchor: Option<String>,
    pub base_uri: String,
    pub resource_uri: String,
    pub resource_root_location: String,
    pub is_resource_root: bool,
}

impl Schema {
    pub fn definition_label<'a>(&self, name: &'a str, definition: &'a Schema) -> &'a str {
        if self.location == self.identity.resource_root_location
            && !name.is_empty()
            && self
                .definitions
                .iter()
                .filter(|(other, _)| other == name)
                .count()
                == 1
        {
            name
        } else {
            &definition.location
        }
    }

    pub fn references(&self) -> impl Iterator<Item = &Reference> {
        [
            self.reference.as_ref(),
            self.recursive_reference.as_ref(),
            self.dynamic_reference.as_ref(),
        ]
        .into_iter()
        .flatten()
    }

    pub fn for_each_child<'a>(&'a self, mut visit: impl FnMut(SchemaChildRole<'a>, &'a Schema)) {
        for (name, definition) in &self.definitions {
            visit(SchemaChildRole::Definition(name), definition);
        }
        for (_, property) in &self.properties {
            visit(SchemaChildRole::Other, property);
        }
        for (_, pattern) in &self.object_constraints.pattern_properties {
            visit(SchemaChildRole::Other, pattern);
        }
        if let Some(property_names) = &self.object_constraints.property_names {
            visit(SchemaChildRole::Other, property_names);
        }
        if let Some(additional) = &self.object_constraints.additional_properties {
            visit(SchemaChildRole::Other, additional);
        }
        if let Some(unevaluated) = &self.object_constraints.unevaluated_properties {
            visit(SchemaChildRole::Other, unevaluated);
        }
        for (_, dependent_schema) in &self.object_constraints.dependent_schemas {
            visit(SchemaChildRole::Other, dependent_schema);
        }
        if let Some(items) = &self.array_constraints.items {
            visit(SchemaChildRole::Other, items);
        }
        if let Some(prefix_items) = &self.array_constraints.prefix_items {
            for item in prefix_items {
                visit(SchemaChildRole::Other, item);
            }
        }
        if let Some(contains) = &self.array_constraints.contains {
            visit(SchemaChildRole::Other, contains);
        }
        if let Some(unevaluated) = &self.array_constraints.unevaluated_items {
            visit(SchemaChildRole::Other, unevaluated);
        }
        for branch in &self.logical_applicators.all_of {
            visit(SchemaChildRole::Other, branch);
        }
        for branch in &self.logical_applicators.any_of {
            visit(SchemaChildRole::Other, branch);
        }
        for branch in &self.logical_applicators.one_of {
            visit(SchemaChildRole::Other, branch);
        }
        if let Some(not) = &self.logical_applicators.not {
            visit(SchemaChildRole::Other, not);
        }
        if let Some(condition) = &self.conditional_applicators.condition {
            visit(SchemaChildRole::Other, condition);
        }
        if let Some(then_branch) = &self.conditional_applicators.then_branch {
            visit(SchemaChildRole::Other, then_branch);
        }
        if let Some(else_branch) = &self.conditional_applicators.else_branch {
            visit(SchemaChildRole::Other, else_branch);
        }
        if let Some(content_schema) = &self.annotations.content_schema {
            visit(SchemaChildRole::Other, content_schema);
        }
    }

    pub fn for_each_child_mut(&mut self, mut visit: impl FnMut(&mut Schema)) {
        for (_, definition) in &mut self.definitions {
            visit(definition);
        }
        for (_, property) in &mut self.properties {
            visit(property);
        }
        for (_, pattern) in &mut self.object_constraints.pattern_properties {
            visit(pattern);
        }
        if let Some(property_names) = &mut self.object_constraints.property_names {
            visit(property_names);
        }
        if let Some(additional) = &mut self.object_constraints.additional_properties {
            visit(additional);
        }
        if let Some(unevaluated) = &mut self.object_constraints.unevaluated_properties {
            visit(unevaluated);
        }
        for (_, dependent_schema) in &mut self.object_constraints.dependent_schemas {
            visit(dependent_schema);
        }
        if let Some(items) = &mut self.array_constraints.items {
            visit(items);
        }
        if let Some(prefix_items) = &mut self.array_constraints.prefix_items {
            for item in prefix_items {
                visit(item);
            }
        }
        if let Some(contains) = &mut self.array_constraints.contains {
            visit(contains);
        }
        if let Some(unevaluated) = &mut self.array_constraints.unevaluated_items {
            visit(unevaluated);
        }
        for branch in &mut self.logical_applicators.all_of {
            visit(branch);
        }
        for branch in &mut self.logical_applicators.any_of {
            visit(branch);
        }
        for branch in &mut self.logical_applicators.one_of {
            visit(branch);
        }
        if let Some(not) = &mut self.logical_applicators.not {
            visit(not);
        }
        if let Some(condition) = &mut self.conditional_applicators.condition {
            visit(condition);
        }
        if let Some(then_branch) = &mut self.conditional_applicators.then_branch {
            visit(then_branch);
        }
        if let Some(else_branch) = &mut self.conditional_applicators.else_branch {
            visit(else_branch);
        }
        if let Some(content_schema) = &mut self.annotations.content_schema {
            visit(content_schema);
        }
    }
}

pub enum SchemaChildRole<'a> {
    Definition(&'a str),
    Other,
}

pub struct Reference {
    pub kind: ReferenceKind,
    pub uri: String,
    pub resolved_uri: Option<String>,
    pub target: ReferenceTarget,
    pub dynamic_start: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ReferenceKind {
    Static,
    Recursive,
    Dynamic,
}

impl ReferenceKind {
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Static => "$ref",
            Self::Recursive => "$recursiveRef",
            Self::Dynamic => "$dynamicRef",
        }
    }
}

pub enum ReferenceTarget {
    Local {
        location: String,
        display_name: String,
        recursive: bool,
    },
    External,
    Unresolved,
}

#[derive(Default)]
pub struct StringConstraints {
    pub min_length: Option<Number>,
    pub max_length: Option<Number>,
    pub pattern: Option<String>,
}

#[derive(Default)]
pub struct ObjectConstraints {
    pub min_properties: Option<Number>,
    pub max_properties: Option<Number>,
    pub pattern_properties: Vec<(String, Schema)>,
    pub property_names: Option<Box<Schema>>,
    pub dependent_required: Vec<(String, Vec<String>)>,
    pub dependent_schemas: Vec<(String, Schema)>,
    pub additional_properties: Option<Box<Schema>>,
    pub unevaluated_properties: Option<Box<Schema>>,
}

#[derive(Default)]
pub struct ArrayConstraints {
    pub items: Option<Box<Schema>>,
    pub prefix_items: Option<Vec<Schema>>,
    pub contains: Option<Box<Schema>>,
    pub min_contains: Option<Number>,
    pub max_contains: Option<Number>,
    pub min_items: Option<Number>,
    pub max_items: Option<Number>,
    pub unique_items: Option<bool>,
    pub unevaluated_items: Option<Box<Schema>>,
}

#[derive(Default)]
pub struct LogicalApplicators {
    pub all_of: Vec<Schema>,
    pub any_of: Vec<Schema>,
    pub one_of: Vec<Schema>,
    pub not: Option<Box<Schema>>,
}

#[derive(Default)]
pub struct ConditionalApplicators {
    pub condition: Option<Box<Schema>>,
    pub then_branch: Option<Box<Schema>>,
    pub else_branch: Option<Box<Schema>>,
}

#[derive(Default)]
pub struct NumberConstraints {
    pub minimum: Option<Number>,
    pub maximum: Option<Number>,
    pub exclusive_minimum: Option<Number>,
    pub exclusive_maximum: Option<Number>,
    pub multiple_of: Option<Number>,
}

#[derive(Default)]
pub struct ValueConstraints {
    pub const_value: Option<Value>,
    pub enum_values: Option<Vec<Value>>,
}

#[derive(Default)]
pub struct Annotations {
    pub title: Option<String>,
    pub description: Option<String>,
    pub default_value: Option<Value>,
    pub examples: Option<Vec<Value>>,
    pub deprecated: Option<bool>,
    pub read_only: Option<bool>,
    pub write_only: Option<bool>,
    pub format: Option<String>,
    pub content_encoding: Option<String>,
    pub content_media_type: Option<String>,
    pub content_schema: Option<Box<Schema>>,
    pub comment: Option<String>,
    pub vocabulary: Vec<(String, bool)>,
}

impl Annotations {
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.description.is_none()
            && self.default_value.is_none()
            && self.examples.is_none()
            && self.deprecated.is_none()
            && self.read_only.is_none()
            && self.write_only.is_none()
            && self.format.is_none()
            && self.content_encoding.is_none()
            && self.content_media_type.is_none()
            && self.content_schema.is_none()
            && self.comment.is_none()
            && self.vocabulary.is_empty()
    }
}

pub enum SchemaKind {
    Any,
    Never,
    Typed(Vec<String>),
    Unspecified,
}
