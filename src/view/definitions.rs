use std::collections::{HashMap, HashSet, VecDeque};

use crate::schema::{ReferenceTarget, Schema, SchemaChildRole};

pub fn referenced_locations<'a>(
    document: &'a Schema,
    root: &'a Schema,
    include_annotations: bool,
) -> HashSet<&'a str> {
    let mut inventory = SchemaInventory::default();
    inventory.insert(document, None);
    inventory.referenced_locations(root, include_annotations)
}

#[derive(Default)]
struct SchemaInventory<'a> {
    schemas: HashMap<&'a str, &'a Schema>,
    definition_owners: HashMap<&'a str, &'a str>,
}

impl<'a> SchemaInventory<'a> {
    fn insert(&mut self, schema: &'a Schema, definition_owner: Option<&'a str>) {
        self.schemas.insert(&schema.location, schema);
        if let Some(owner) = definition_owner {
            self.definition_owners.insert(&schema.location, owner);
        }
        schema.for_each_child(|role, child| {
            let child_owner = match role {
                SchemaChildRole::Definition(_) => Some(child.location.as_str()),
                _ => definition_owner,
            };
            self.insert(child, child_owner);
        });
    }

    fn referenced_locations(
        &self,
        root: &'a Schema,
        include_annotations: bool,
    ) -> HashSet<&'a str> {
        let mut included = HashSet::new();
        let mut visited = HashSet::new();
        let mut pending = VecDeque::from([root]);

        while let Some(schema) = pending.pop_front() {
            if !visited.insert(schema.location.as_str()) {
                continue;
            }

            for reference in schema.references() {
                let ReferenceTarget::Local { location, .. } = &reference.target else {
                    continue;
                };
                let Some(target) = self.schemas.get(location.as_str()).copied() else {
                    continue;
                };
                if let Some(owner) = self
                    .definition_owners
                    .get(target.location.as_str())
                    .copied()
                {
                    if owner != root.location {
                        included.insert(owner);
                        pending.push_back(
                            self.schemas
                                .get(owner)
                                .copied()
                                .expect("a definition owner should be indexed"),
                        );
                    }
                } else {
                    pending.push_back(target);
                }
            }

            schema.for_each_child(|role, child| {
                if !matches!(role, SchemaChildRole::Definition(_))
                    && (include_annotations || !matches!(role, SchemaChildRole::ContentSchema))
                {
                    pending.push_back(child);
                }
            });
        }

        included
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::{schema, view::definitions::referenced_locations};

    #[test]
    fn follows_references_transitively_without_entering_unused_definitions() {
        let document = schema::parse(
            r##"{
                "properties": {
                    "part": {"$ref": "#/$defs/A/properties/value"}
                },
                "$defs": {
                    "A": {
                        "properties": {
                            "value": {"$ref": "#/$defs/B"}
                        }
                    },
                    "B": {"$ref": "#/$defs/A"},
                    "Unused": {"$ref": "#/$defs/B"}
                }
            }"##,
            None,
        )
        .expect("schema should parse");

        let locations = referenced_locations(&document.root, &document.root, true);

        assert_eq!(locations, HashSet::from(["#/$defs/A", "#/$defs/B"]));
    }

    #[test]
    fn optionally_excludes_references_from_content_schemas() {
        let document = schema::parse(
            r##"{
                "properties": {
                    "visible": {"$ref": "#/$defs/Visible"}
                },
                "contentSchema": {"$ref": "#/$defs/AnnotationOnly"},
                "$defs": {
                    "Visible": {"type": "integer"},
                    "AnnotationOnly": {"type": "string"}
                }
            }"##,
            None,
        )
        .expect("schema should parse");

        assert_eq!(
            referenced_locations(&document.root, &document.root, true),
            HashSet::from(["#/$defs/Visible", "#/$defs/AnnotationOnly"]),
        );
        assert_eq!(
            referenced_locations(&document.root, &document.root, false),
            HashSet::from(["#/$defs/Visible"]),
        );
    }
}
