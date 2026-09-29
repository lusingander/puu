use std::collections::{HashMap, HashSet};

use fluent_uri::{Uri, UriRef};

use crate::{
    dialect::Dialect,
    schema::{
        error::SchemaError,
        model::{Reference, ReferenceKind, ReferenceTarget, Schema, SchemaChildRole},
    },
};

const INITIAL_BASE_URI: &str = "puu:/document";

struct RegistryEntry {
    location: String,
    display_name: String,
    dynamic_anchor: bool,
    recursive_anchor: bool,
}

#[derive(Clone, Copy, Default)]
struct RegistryEntryKind {
    dynamic_anchor: bool,
    recursive_anchor: bool,
}

#[derive(Default)]
struct SchemaRegistry {
    entries: HashMap<String, RegistryEntry>,
    resources: HashSet<String>,
}

#[derive(Clone)]
struct ActiveResource {
    uri: String,
    root_location: String,
}

pub fn prepare(schema: &mut Schema) -> Result<(), SchemaError> {
    assign_resource_context(schema, INITIAL_BASE_URI, INITIAL_BASE_URI, "#", true)?;
    let mut registry = SchemaRegistry::default();
    collect_registry(schema, "root", &mut Vec::new(), &mut registry)?;
    resolve_references(schema, &registry)
}

fn assign_resource_context(
    schema: &mut Schema,
    inherited_base_uri: &str,
    inherited_resource_uri: &str,
    inherited_resource_root_location: &str,
    document_root: bool,
) -> Result<(), SchemaError> {
    let location = schema.location.clone();
    assign_resource_context_at(
        schema,
        inherited_base_uri,
        inherited_resource_uri,
        inherited_resource_root_location,
        document_root,
    )
    .map_err(|error| error.at(&location))
}

fn assign_resource_context_at(
    schema: &mut Schema,
    inherited_base_uri: &str,
    inherited_resource_uri: &str,
    inherited_resource_root_location: &str,
    document_root: bool,
) -> Result<(), SchemaError> {
    let mut base_uri = inherited_base_uri.to_owned();
    let mut resource_uri = inherited_resource_uri.to_owned();
    let mut resource_root_location = inherited_resource_root_location.to_owned();
    let mut is_resource_root = document_root;

    if let Some(identifier) = schema.identity.identifier.clone() {
        let legacy_anchor = schema.dialect == Dialect::Draft7
            && identifier.starts_with('#')
            && identifier.len() > 1;
        if legacy_anchor {
            let anchor = identifier
                .strip_prefix('#')
                .expect("a legacy anchor starts with '#'");
            if !valid_anchor(Dialect::Draft7, anchor) {
                return Err(SchemaError::InvalidKeywordValue("$id"));
            }
            schema.identity.anchor = Some(anchor.to_owned());
        } else {
            let resolved = resolve_uri_reference(inherited_base_uri, &identifier, "$id")?;
            let parsed = Uri::parse(resolved.as_str())
                .expect("a resolved URI has already been parsed successfully");
            if parsed
                .fragment()
                .is_some_and(|fragment| !fragment.is_empty())
            {
                return Err(SchemaError::InvalidKeywordValue("$id"));
            }
            base_uri = parsed.strip_fragment().to_string();
            resource_uri.clone_from(&base_uri);
            resource_root_location = schema.location.clone();
            is_resource_root = true;
        }
    }
    if let Some(anchor) = &schema.identity.anchor
        && !valid_anchor(schema.dialect, anchor)
    {
        return Err(SchemaError::InvalidKeywordValue("$anchor"));
    }
    if let Some(anchor) = &schema.identity.dynamic_anchor
        && !valid_anchor(Dialect::Draft202012, anchor)
    {
        return Err(SchemaError::InvalidKeywordValue("$dynamicAnchor"));
    }

    schema.identity.base_uri.clone_from(&base_uri);
    schema.identity.resource_uri.clone_from(&resource_uri);
    schema
        .identity
        .resource_root_location
        .clone_from(&resource_root_location);
    schema.identity.is_resource_root = is_resource_root;

    let mut result = Ok(());
    schema.for_each_child_mut(|child| {
        if result.is_ok() {
            result = assign_resource_context(
                child,
                &base_uri,
                &resource_uri,
                &resource_root_location,
                false,
            );
        }
    });
    result
}

fn collect_registry(
    schema: &Schema,
    label: &str,
    active_resources: &mut Vec<ActiveResource>,
    registry: &mut SchemaRegistry,
) -> Result<(), SchemaError> {
    collect_registry_at(schema, label, active_resources, registry)
        .map_err(|error| error.at(&schema.location))
}

fn collect_registry_at(
    schema: &Schema,
    label: &str,
    active_resources: &mut Vec<ActiveResource>,
    registry: &mut SchemaRegistry,
) -> Result<(), SchemaError> {
    for resource in active_resources.iter() {
        let pointer = relative_schema_pointer(&resource.root_location, &schema.location)
            .expect("an active resource must contain its descendant schema");
        let uri = schema_pointer_uri(&resource.uri, pointer);
        register_schema(
            registry,
            uri,
            &schema.location,
            label,
            "$id",
            RegistryEntryKind::default(),
        )?;
    }

    let pushed_resource = schema.identity.is_resource_root;
    if pushed_resource {
        let resource_uri = schema.identity.resource_uri.clone();
        registry.resources.insert(resource_uri.clone());
        let display_name = if schema.location == "#" {
            "root"
        } else {
            &resource_uri
        };
        register_schema(
            registry,
            resource_uri.clone(),
            &schema.location,
            display_name,
            "$id",
            RegistryEntryKind {
                recursive_anchor: schema.identity.recursive_anchor == Some(true),
                ..RegistryEntryKind::default()
            },
        )?;
        register_schema(
            registry,
            format!("{resource_uri}#"),
            &schema.location,
            display_name,
            "$id",
            RegistryEntryKind {
                recursive_anchor: schema.identity.recursive_anchor == Some(true),
                ..RegistryEntryKind::default()
            },
        )?;
        active_resources.push(ActiveResource {
            uri: resource_uri,
            root_location: schema.location.clone(),
        });
    }

    if let Some(anchor) = &schema.identity.anchor {
        let resource = active_resources
            .last()
            .expect("every schema belongs to a resource");
        register_schema(
            registry,
            format!("{}#{anchor}", resource.uri),
            &schema.location,
            label,
            "$anchor",
            RegistryEntryKind::default(),
        )?;
    }
    if let Some(anchor) = &schema.identity.dynamic_anchor {
        let resource = active_resources
            .last()
            .expect("every schema belongs to a resource");
        register_schema(
            registry,
            format!("{}#{anchor}", resource.uri),
            &schema.location,
            label,
            "$dynamicAnchor",
            RegistryEntryKind {
                dynamic_anchor: true,
                ..RegistryEntryKind::default()
            },
        )?;
    }

    let mut result = Ok(());
    schema.for_each_child(|role, child| {
        if result.is_ok() {
            let label = match role {
                SchemaChildRole::Definition(name) => schema.definition_label(name, child),
                _ => &child.location,
            };
            result = collect_registry(child, label, active_resources, registry);
        }
    });
    if pushed_resource {
        active_resources.pop();
    }
    result
}

fn register_schema(
    registry: &mut SchemaRegistry,
    uri: String,
    location: &str,
    display_name: &str,
    duplicate_keyword: &'static str,
    entry_kind: RegistryEntryKind,
) -> Result<(), SchemaError> {
    if let Some(existing) = registry.entries.get(&uri) {
        if existing.location != location || existing.dynamic_anchor || entry_kind.dynamic_anchor {
            return Err(SchemaError::InvalidKeywordValue(duplicate_keyword));
        }
        return Ok(());
    }
    registry.entries.insert(
        uri,
        RegistryEntry {
            location: location.to_owned(),
            display_name: display_name.to_owned(),
            dynamic_anchor: entry_kind.dynamic_anchor,
            recursive_anchor: entry_kind.recursive_anchor,
        },
    );
    Ok(())
}

fn valid_anchor(dialect: Dialect, anchor: &str) -> bool {
    let mut characters = anchor.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    let valid_first =
        first.is_ascii_alphabetic() || (dialect == Dialect::Draft202012 && first == '_');
    valid_first
        && characters.all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(character, '-' | '_' | '.')
                || (dialect != Dialect::Draft202012 && character == ':')
        })
}

fn resolve_references(schema: &mut Schema, registry: &SchemaRegistry) -> Result<(), SchemaError> {
    let location = schema.location.clone();
    resolve_references_at(schema, registry).map_err(|error| error.at(&location))
}

fn resolve_references_at(
    schema: &mut Schema,
    registry: &SchemaRegistry,
) -> Result<(), SchemaError> {
    let Schema {
        location,
        identity,
        reference,
        recursive_reference,
        dynamic_reference,
        ..
    } = schema;
    for reference in [reference, recursive_reference, dynamic_reference]
        .into_iter()
        .filter_map(Option::as_mut)
    {
        resolve_reference(reference, &identity.base_uri, location, registry)?;
    }
    let mut result = Ok(());
    schema.for_each_child_mut(|child| {
        if result.is_ok() {
            result = resolve_references(child, registry);
        }
    });
    result
}

fn resolve_reference(
    reference: &mut Reference,
    base_uri: &str,
    schema_location: &str,
    registry: &SchemaRegistry,
) -> Result<(), SchemaError> {
    let resolved = resolve_uri_reference(base_uri, &reference.uri, reference.kind.keyword())?;
    reference.resolved_uri = Some(resolved.clone());
    let lookup_uri = canonical_reference_uri(&resolved).unwrap_or_else(|| resolved.clone());
    reference.target = if let Some(target) = registry.entries.get(&lookup_uri) {
        reference.dynamic_start = match reference.kind {
            ReferenceKind::Static => false,
            ReferenceKind::Recursive => target.recursive_anchor,
            ReferenceKind::Dynamic => target.dynamic_anchor,
        };
        ReferenceTarget::Local {
            recursive: schema_location == target.location
                || schema_location.starts_with(&format!("{}/", target.location)),
            location: target.location.clone(),
            display_name: target.display_name.clone(),
        }
    } else {
        let resource_uri = Uri::parse(resolved.as_str())
            .expect("a resolved URI has already been parsed successfully")
            .strip_fragment()
            .to_string();
        if registry.resources.contains(&resource_uri) {
            ReferenceTarget::Unresolved
        } else {
            ReferenceTarget::External
        }
    };
    Ok(())
}

fn resolve_uri_reference(
    base_uri: &str,
    reference: &str,
    keyword: &'static str,
) -> Result<String, SchemaError> {
    let base = Uri::parse(base_uri).map_err(|_| SchemaError::InvalidKeywordValue(keyword))?;
    let reference =
        UriRef::parse(reference).map_err(|_| SchemaError::InvalidKeywordValue(keyword))?;
    let resolved = reference
        .resolve_against(&base)
        .map_err(|_| SchemaError::InvalidKeywordValue(keyword))?;
    Ok(resolved.normalize().to_string())
}

fn canonical_reference_uri(uri: &str) -> Option<String> {
    let parsed = Uri::parse(uri).ok()?;
    let fragment = parsed.fragment()?;
    let pointer = canonical_local_pointer(&format!("#{fragment}"))?;
    Some(schema_pointer_uri(
        parsed.strip_fragment().as_ref(),
        pointer.strip_prefix('#')?,
    ))
}

fn relative_schema_pointer<'a>(resource_root: &str, location: &'a str) -> Option<&'a str> {
    if resource_root == location {
        return Some("");
    }
    location.strip_prefix(resource_root)
}

fn schema_pointer_uri(resource_uri: &str, pointer: &str) -> String {
    format!("{resource_uri}#{}", encode_uri_fragment(pointer))
}

fn encode_uri_fragment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'-' | b'.'
                    | b'_'
                    | b'~'
                    | b'/'
                    | b'?'
                    | b':'
                    | b'@'
                    | b'!'
                    | b'$'
                    | b'&'
                    | b'\''
                    | b'('
                    | b')'
                    | b'*'
                    | b'+'
                    | b','
                    | b';'
                    | b'='
            )
        {
            encoded.push(char::from(byte));
        } else {
            use std::fmt::Write;
            write!(encoded, "%{byte:02X}").expect("writing to String");
        }
    }
    encoded
}

pub fn canonical_local_pointer(uri: &str) -> Option<String> {
    let fragment = uri.strip_prefix('#')?;
    let mut decoded_bytes = Vec::with_capacity(fragment.len());
    let mut bytes = fragment.bytes();
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let high = (bytes.next()? as char).to_digit(16)?;
            let low = (bytes.next()? as char).to_digit(16)?;
            decoded_bytes.push((high * 16 + low) as u8);
        } else {
            decoded_bytes.push(byte);
        }
    }
    let pointer = String::from_utf8(decoded_bytes).ok()?;
    if pointer.is_empty() {
        return Some("#".to_owned());
    }
    let tokens = pointer.strip_prefix('/')?;
    let mut canonical = String::from("#");
    for token in tokens.split('/') {
        let mut decoded_token = String::new();
        let mut chars = token.chars();
        while let Some(character) = chars.next() {
            if character == '~' {
                decoded_token.push(match chars.next()? {
                    '0' => '~',
                    '1' => '/',
                    _ => return None,
                });
            } else {
                decoded_token.push(character);
            }
        }
        canonical.push('/');
        canonical.push_str(&decoded_token.replace('~', "~0").replace('/', "~1"));
    }
    Some(canonical)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::schema::resource::canonical_local_pointer;

    #[rustfmt::skip]
    #[rstest]
    #[case("#", Some("#"))]
    #[case("#/$defs/", Some("#/$defs/"))]
    #[case("#/$defs/a~1b", Some("#/$defs/a~1b"))]
    #[case("#/$defs/~01", Some("#/$defs/~01"))]
    #[case("#/%24defs/caf%C3%A9", Some("#/$defs/café"))]
    #[case("#/$defs/a~2b", None)]
    #[case("#/$defs/%GG", None)]
    #[case("#/%FF", None)]
    #[case("#Value", None)]
    fn normalizes_local_json_pointers(#[case] uri: &str, #[case] expected: Option<&str>) {
        assert_eq!(canonical_local_pointer(uri).as_deref(), expected);
    }
}
