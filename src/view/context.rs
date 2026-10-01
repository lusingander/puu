use std::collections::HashSet;

use crate::{
    schema::SchemaIndex,
    view::{ViewNode, depth::omission_node, model::ViewOptions},
};

const MAX_EXPANSION_DEPTH: usize = 64;
const MAX_EXPANDED_NODES: usize = 10_000;

pub struct BuildContext<'schema, 'options> {
    pub index: &'options SchemaIndex<'schema>,
    pub options: &'options ViewOptions,
    pub depth: usize,
    pub active_schemas: HashSet<String>,
    pub expansion_depth: usize,
    expanded_nodes: usize,
}

impl<'schema, 'options> BuildContext<'schema, 'options> {
    pub fn new(index: &'options SchemaIndex<'schema>, options: &'options ViewOptions) -> Self {
        Self {
            index,
            options,
            depth: 0,
            active_schemas: HashSet::new(),
            expansion_depth: 0,
            expanded_nodes: 0,
        }
    }

    pub fn beyond_max_depth(&self) -> bool {
        self.options.max_depth.is_some_and(|max| self.depth > max)
    }

    pub fn expansion_stop_reason(&self) -> Option<&'static str> {
        if self.depth > MAX_EXPANSION_DEPTH {
            Some("expansion stopped: depth limit")
        } else if self.expanded_nodes >= MAX_EXPANDED_NODES {
            Some("expansion stopped: node limit")
        } else {
            None
        }
    }

    pub fn begin_node(&mut self) -> Option<ViewNode> {
        if self.beyond_max_depth() {
            return Some(omission_node());
        }
        if self.expansion_depth > 0 {
            if let Some(reason) = self.expansion_stop_reason() {
                return Some(crate::view::depth::omission_with_reason(reason));
            }
            self.expanded_nodes += 1;
        }
        None
    }

    pub fn at_next_depth<T>(&mut self, build: impl FnOnce(&mut Self) -> T) -> T {
        self.depth += 1;
        let result = build(self);
        self.depth -= 1;
        result
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        schema,
        view::{
            build::schema_node,
            model::{ViewNodeRole, ViewOptions},
        },
    };

    use super::BuildContext;

    #[test]
    fn does_not_construct_reference_targets_below_the_display_limit() {
        let parsed = schema::parse(
            r##"{
            "$ref": "#/$defs/A", "$defs": {"A": {"$ref": "#/$defs/B"}, "B": true}
        }"##,
            None,
        )
        .expect("schema should parse");
        let index = schema::SchemaIndex::new(&parsed);
        let options = ViewOptions {
            expand_refs: true,
            max_depth: Some(0),
            ..ViewOptions::default()
        };
        let mut context = BuildContext::new(&index, &options);
        let root = schema_node(&parsed.root, "root", false, true, &mut context);

        assert_eq!(context.expanded_nodes, 0);
        assert!(context.active_schemas.is_empty());
        assert_eq!(context.expansion_depth, 0);
        assert_eq!(context.depth, 0);
        assert!(matches!(root.children[0].role, ViewNodeRole::Omission));
    }
}
