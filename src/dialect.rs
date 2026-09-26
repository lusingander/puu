use std::str::FromStr;

#[derive(clap::ValueEnum, Clone, Copy, Debug, Eq, PartialEq)]
pub enum Dialect {
    #[value(name = "2020-12")]
    Draft202012,
    #[value(name = "2019-09")]
    Draft201909,
    #[value(name = "7")]
    Draft7,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArrayItemsSyntax {
    PrefixItems,
    ItemsArray,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectDependenciesSyntax {
    Split,
    Combined,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DynamicReferenceSyntax {
    Dynamic,
    Recursive,
    None,
}

impl Dialect {
    pub const fn latest_supported() -> Self {
        Self::Draft202012
    }

    pub fn from_schema_uri(uri: &str) -> Option<Self> {
        match uri {
            "https://json-schema.org/draft/2020-12/schema" => Some(Self::Draft202012),
            "https://json-schema.org/draft/2019-09/schema"
            | "http://json-schema.org/draft/2019-09/schema" => Some(Self::Draft201909),
            "http://json-schema.org/draft-07/schema#"
            | "https://json-schema.org/draft-07/schema#" => Some(Self::Draft7),
            _ => None,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Draft202012 => "2020-12",
            Self::Draft201909 => "2019-09",
            Self::Draft7 => "Draft 7",
        }
    }

    pub const fn definition_keywords(self) -> &'static [&'static str] {
        match self {
            Self::Draft202012 | Self::Draft201909 => &["definitions", "$defs"],
            Self::Draft7 => &["definitions"],
        }
    }

    pub const fn array_items_syntax(self) -> ArrayItemsSyntax {
        match self {
            Self::Draft202012 => ArrayItemsSyntax::PrefixItems,
            Self::Draft201909 | Self::Draft7 => ArrayItemsSyntax::ItemsArray,
        }
    }

    pub const fn allows_ref_siblings(self) -> bool {
        match self {
            Self::Draft202012 | Self::Draft201909 => true,
            Self::Draft7 => false,
        }
    }

    pub const fn object_dependencies_syntax(self) -> ObjectDependenciesSyntax {
        match self {
            Self::Draft202012 | Self::Draft201909 => ObjectDependenciesSyntax::Split,
            Self::Draft7 => ObjectDependenciesSyntax::Combined,
        }
    }

    pub const fn supports_contains_counts(self) -> bool {
        match self {
            Self::Draft202012 | Self::Draft201909 => true,
            Self::Draft7 => false,
        }
    }

    pub const fn supports_deprecated(self) -> bool {
        !matches!(self, Self::Draft7)
    }

    pub const fn supports_content_schema(self) -> bool {
        !matches!(self, Self::Draft7)
    }

    pub const fn supports_vocabulary(self) -> bool {
        !matches!(self, Self::Draft7)
    }

    pub const fn supports_unevaluated(self) -> bool {
        !matches!(self, Self::Draft7)
    }

    pub const fn supports_anchor(self) -> bool {
        !matches!(self, Self::Draft7)
    }

    pub const fn dynamic_reference_syntax(self) -> DynamicReferenceSyntax {
        match self {
            Self::Draft202012 => DynamicReferenceSyntax::Dynamic,
            Self::Draft201909 => DynamicReferenceSyntax::Recursive,
            Self::Draft7 => DynamicReferenceSyntax::None,
        }
    }

    pub const fn contains_marks_evaluated_items(self) -> bool {
        matches!(self, Self::Draft202012)
    }
}

impl FromStr for Dialect {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        <Self as clap::ValueEnum>::from_str(value, false)
            .map_err(|_| format!("unsupported draft {value:?}; expected 2020-12, 2019-09, or 7"))
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::dialect::{
        ArrayItemsSyntax, Dialect, DynamicReferenceSyntax, ObjectDependenciesSyntax,
    };

    #[rustfmt::skip]
    #[rstest]
    #[case(Dialect::Draft202012, &["definitions", "$defs"], ArrayItemsSyntax::PrefixItems, ObjectDependenciesSyntax::Split, true, true, DynamicReferenceSyntax::Dynamic)]
    #[case(Dialect::Draft201909, &["definitions", "$defs"], ArrayItemsSyntax::ItemsArray, ObjectDependenciesSyntax::Split, true, false, DynamicReferenceSyntax::Recursive)]
    #[case(Dialect::Draft7, &["definitions"], ArrayItemsSyntax::ItemsArray, ObjectDependenciesSyntax::Combined, false, false, DynamicReferenceSyntax::None)]
    fn exposes_keyword_rules(
        #[case] dialect: Dialect,
        #[case] definitions: &[&str],
        #[case] array_items: ArrayItemsSyntax,
        #[case] object_dependencies: ObjectDependenciesSyntax,
        #[case] modern_keywords: bool,
        #[case] contains_marks_evaluated_items: bool,
        #[case] dynamic_reference_syntax: DynamicReferenceSyntax,
    ) {
        assert_eq!(dialect.definition_keywords(), definitions);
        assert_eq!(dialect.array_items_syntax(), array_items);
        assert_eq!(
            dialect.object_dependencies_syntax(),
            object_dependencies
        );
        assert_eq!(dialect.allows_ref_siblings(), modern_keywords);
        assert_eq!(dialect.supports_contains_counts(), modern_keywords);
        assert_eq!(dialect.supports_deprecated(), modern_keywords);
        assert_eq!(dialect.supports_content_schema(), modern_keywords);
        assert_eq!(dialect.supports_vocabulary(), modern_keywords);
        assert_eq!(dialect.supports_unevaluated(), modern_keywords);
        assert_eq!(dialect.supports_anchor(), modern_keywords);
        assert_eq!(
            dialect.dynamic_reference_syntax(),
            dynamic_reference_syntax
        );
        assert_eq!(
            dialect.contains_marks_evaluated_items(),
            contains_marks_evaluated_items
        );
    }
}
