use crate::view::model::{ViewDetail, ViewDocument, ViewNode, ViewNodeRole};

pub fn limit(document: &mut ViewDocument, max_depth: usize) {
    for root in &mut document.roots {
        limit_node(root, 0, max_depth);
    }
}

fn limit_node(node: &mut ViewNode, depth: usize, max_depth: usize) {
    if depth == max_depth {
        if !node.children.is_empty() {
            node.children = vec![omission_node()];
        }
        return;
    }

    for child in &mut node.children {
        limit_node(child, depth + 1, max_depth);
    }
}

pub(super) fn omission_node() -> ViewNode {
    omission_with_reason("children omitted")
}

pub(super) fn omission_with_reason(reason: &str) -> ViewNode {
    ViewNode {
        role: ViewNodeRole::Omission,
        key: None,
        value: "…".to_owned(),
        details: vec![ViewDetail::marker(reason)],
        children: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use crate::view::model::{ViewDocument, ViewNode, ViewNodeRole};

    use super::limit;

    #[test]
    fn replaces_children_below_the_maximum_depth() {
        let mut document = ViewDocument {
            roots: vec![node(
                "root",
                vec![node("child", vec![node("leaf", vec![])])],
            )],
        };

        limit(&mut document, 1);

        let child = &document.roots[0].children[0];
        assert_eq!(child.value, "child");
        assert_eq!(child.children.len(), 1);
        assert!(matches!(child.children[0].role, ViewNodeRole::Omission));
        assert_eq!(child.children[0].value, "…");
        assert_eq!(child.children[0].details[0].text, "children omitted");
    }

    #[test]
    fn treats_each_root_as_depth_zero() {
        let mut document = ViewDocument {
            roots: vec![
                node("root", vec![node("child", vec![])]),
                node("Definitions", vec![node("definition", vec![])]),
            ],
        };

        limit(&mut document, 0);

        for root in &document.roots {
            assert_eq!(root.children.len(), 1);
            assert!(matches!(root.children[0].role, ViewNodeRole::Omission));
        }
    }

    #[test]
    fn does_not_add_an_omission_to_a_leaf() {
        let mut document = ViewDocument {
            roots: vec![node("root", Vec::new())],
        };

        limit(&mut document, 0);

        assert!(document.roots[0].children.is_empty());
    }

    fn node(value: &str, children: Vec<ViewNode>) -> ViewNode {
        ViewNode {
            role: ViewNodeRole::Section,
            key: None,
            value: value.to_owned(),
            details: Vec::new(),
            children,
        }
    }
}
