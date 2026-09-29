use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt,
};

use crate::schema::{ParsedDocument, Schema, SchemaChildRole, resource::canonical_local_pointer};

pub struct SchemaIndexEntry<'a> {
    pub schema: &'a Schema,
    pub display_name: String,
}

pub struct SchemaIndex<'a> {
    by_location: HashMap<&'a str, SchemaIndexEntry<'a>>,
    value_locations: &'a HashSet<String>,
}

impl<'a> SchemaIndex<'a> {
    pub fn new(document: &'a ParsedDocument) -> Self {
        let mut index = Self {
            by_location: HashMap::new(),
            value_locations: &document.value_locations,
        };
        index.insert(&document.root, "root".to_owned());
        index
    }

    pub fn get(&self, location: &str) -> Option<&SchemaIndexEntry<'a>> {
        self.by_location.get(location)
    }

    pub fn select(&self, pointer: &str) -> Result<&SchemaIndexEntry<'a>, SchemaPointerError> {
        let fragment = if pointer.starts_with('/') {
            Cow::Owned(format!("#{pointer}"))
        } else {
            Cow::Borrowed(pointer)
        };
        let location = canonical_local_pointer(&fragment)
            .ok_or_else(|| SchemaPointerError::Invalid(pointer.to_owned()))?;
        if let Some(entry) = self.get(&location) {
            return Ok(entry);
        }
        if self.value_locations.contains(&location) {
            Err(SchemaPointerError::NotSchema(pointer.to_owned()))
        } else {
            Err(SchemaPointerError::NotFound(pointer.to_owned()))
        }
    }

    fn insert(&mut self, schema: &'a Schema, display_name: String) {
        self.by_location.insert(
            &schema.location,
            SchemaIndexEntry {
                schema,
                display_name,
            },
        );
        schema.for_each_child(|role, child| {
            self.insert(child, child_display_name(role));
        });
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum SchemaPointerError {
    Invalid(String),
    NotFound(String),
    NotSchema(String),
}

impl fmt::Display for SchemaPointerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(pointer) => write!(formatter, "invalid schema pointer: {pointer:?}"),
            Self::NotFound(pointer) => {
                write!(formatter, "schema pointer not found: {pointer:?}")
            }
            Self::NotSchema(pointer) => {
                write!(formatter, "pointer {pointer:?} does not identify a schema")
            }
        }
    }
}

fn child_display_name(role: SchemaChildRole<'_>) -> String {
    match role {
        SchemaChildRole::Definition(name) | SchemaChildRole::Property(name) => name.to_owned(),
        SchemaChildRole::PatternProperty(pattern) => {
            format!("<properties matching {pattern:?}>")
        }
        SchemaChildRole::PropertyNames => "<property names>".to_owned(),
        SchemaChildRole::AdditionalProperties => "<other properties>".to_owned(),
        SchemaChildRole::UnevaluatedProperties => "unevaluated properties".to_owned(),
        SchemaChildRole::DependentSchema(name) => {
            format!("when property {name:?} exists")
        }
        SchemaChildRole::PrefixItem(index) => format!("[{index}]"),
        SchemaChildRole::Items => "items".to_owned(),
        SchemaChildRole::Contains => "contains".to_owned(),
        SchemaChildRole::UnevaluatedItems => "unevaluated items".to_owned(),
        SchemaChildRole::LogicalBranch { keyword, index } => format!("{keyword}[{index}]"),
        SchemaChildRole::Not => "not".to_owned(),
        SchemaChildRole::Condition => "if".to_owned(),
        SchemaChildRole::Then => "then".to_owned(),
        SchemaChildRole::Else => "else".to_owned(),
        SchemaChildRole::ContentSchema => "content schema".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use crate::schema::{SchemaIndex, index::SchemaPointerError, parse};

    #[test]
    fn indexes_schemas_with_display_names() {
        let document = parse(
            r##"{
                "$defs": {"User": {"type": "object"}},
                "properties": {"user": {"$ref": "#/$defs/User"}},
                "allOf": [{"type": "string"}]
            }"##,
            None,
        )
        .expect("schema should parse");
        let index = SchemaIndex::new(&document);

        assert_eq!(
            index
                .get("#/$defs/User")
                .expect("definition should be indexed")
                .display_name,
            "User"
        );
        assert_eq!(
            index
                .get("#/properties/user")
                .expect("property should be indexed")
                .display_name,
            "user"
        );
        assert_eq!(
            index
                .get("#/allOf/0")
                .expect("branch should be indexed")
                .display_name,
            "allOf[0]"
        );
    }

    #[test]
    fn selects_schema_pointers_and_classifies_failures() {
        let document = parse(
            r##"{
                "$defs": {"café": {"type": "string"}},
                "properties": {"a/b": {"type": "integer"}}
            }"##,
            None,
        )
        .expect("schema should parse");
        let index = SchemaIndex::new(&document);

        assert_eq!(
            index
                .select("/%24defs/caf%C3%A9")
                .expect("encoded pointer should select the definition")
                .schema
                .location,
            "#/$defs/café"
        );
        assert_eq!(
            index
                .select("#/properties/a~1b")
                .expect("escaped pointer should select the property")
                .schema
                .location,
            "#/properties/a~1b"
        );
        assert_eq!(
            index
                .select("#/properties/a~1b/type")
                .err()
                .expect("type value should not be a schema"),
            SchemaPointerError::NotSchema("#/properties/a~1b/type".to_owned())
        );
        assert_eq!(
            index
                .select("#/missing")
                .err()
                .expect("missing pointer should fail"),
            SchemaPointerError::NotFound("#/missing".to_owned())
        );
        assert_eq!(
            index
                .select("#anchor")
                .err()
                .expect("anchor syntax should be invalid"),
            SchemaPointerError::Invalid("#anchor".to_owned())
        );
    }
}
