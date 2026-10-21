# Supported scope

Puu supports JSON Schema Draft 2020-12, 2019-09, and Draft 7. It assumes Draft 2020-12 when `$schema` is absent. This is not a complete implementation of each draft; it supports the parts needed to render schemas, not validate instances.

## Supported keywords

- Boolean schemas, `type`, `enum`, and `const`
- `properties`, `required`, `additionalProperties`, `patternProperties`, and `propertyNames`
- `items`, `prefixItems`, `additionalItems`, and `contains`
- Common string, numeric, object, and array constraints
- `allOf`, `anyOf`, `oneOf`, `not`, and `if` / `then` / `else`
- Property and schema dependencies, `unevaluatedProperties`, and `unevaluatedItems`
- `$id`, `$defs` / `definitions`, `$ref`, anchors, and recursive or dynamic references
- Annotations such as `title`, `description`, `default`, `examples`, `readOnly`, `writeOnly`, and the content-related keywords

## References

References within the input document are resolved for display. External references are shown as external but are not fetched.

## Keyword display

Unrecognized keywords are reported as `[uninterpreted: ...]`. Keywords that are recognized but have no effect in their current context are reported as `[ignored: ...]`. With `--verbose`, uninterpreted keyword values and expanded annotation details are included.

`format` is displayed as an annotation; instance values are not validated against it.
