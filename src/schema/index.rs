use std::collections::HashMap;

use crate::schema::{Schema, SchemaChildRole};

pub struct SchemaIndexEntry<'a> {
    pub schema: &'a Schema,
    pub display_name: String,
}

pub struct SchemaIndex<'a> {
    by_location: HashMap<&'a str, SchemaIndexEntry<'a>>,
}

impl<'a> SchemaIndex<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        let mut index = Self {
            by_location: HashMap::new(),
        };
        index.insert(schema, "root".to_owned());
        index
    }

    pub fn get(&self, location: &str) -> Option<&SchemaIndexEntry<'a>> {
        self.by_location.get(location)
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
    use crate::schema::{SchemaIndex, parse};

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
}
