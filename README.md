# Puu

Render JSON Schema for humans 🌳

## About

A CLI that transforms JSON Schema into readable terminal views.
Built for understanding, sharing, and documenting schemas — not validating instances.

```
$ puu ./tests/fixtures/render/order-model.json
root object [closed] [title: "Order"] [description: "A customer order."]
├─ orderId string [required] [read only] [format: uuid]
├─ status string [required] [enum: "pending" | "paid" | "shipped"]
├─ customer object [required] [closed]
│  ├─ name string [required] [1..80 chars]
│  └─ email string [required] [format: email]
└─ ...
```

<img src="./img/order-model.png">

For more input examples and their rendered output, see the [fixture output catalog](docs/render-fixtures.md).

## Installation

### [Cargo](https://crates.io/crates/puu)

```
cargo install --locked puu
```

### Downloading a binary

Pre-built binaries are available from the [releases page](https://github.com/lusingander/puu/releases).

## Usage

```
puu [OPTIONS] <SCHEMA>
```

`SCHEMA` is the path or HTTP(S) URL of a JSON Schema, or `-` to read from standard input.

```
puu schema.json
puu https://json.schemastore.org/package.json
puu - < schema.json
puu --draft 7 schema.json
puu --pointer '#/$defs/User' schema.json
puu --max-depth 2 schema.json
puu --definitions referenced schema.json
puu -r --definitions none schema.json
```

### Options

```
Puu - Render JSON Schema for humans 🌳

Usage: puu [OPTIONS] <SCHEMA>

Arguments:
  <SCHEMA>  JSON Schema file or URL to render, or '-' to read from standard input

Options:
  -d, --draft <DRAFT>       Override the JSON Schema draft [possible values: 2020-12, 2019-09, 7]
  -p, --pointer <POINTER>   Render the schema at a JSON Pointer
  -L, --max-depth <N>       Limit display tree depth, counting the root as depth 0
  -D, --definitions <MODE>  Control which definitions are displayed [default: all] [possible values: all, referenced, none]
  -r, --expand-refs         Expand locally resolved schema references
  -v, --verbose             Show full annotations and uninterpreted values
  -c, --color <COLOR>       Control colored tree output [default: auto] [possible values: auto, always, never]
  -h, --help                Print help
  -V, --version             Print version
```

### Pointer selection

`--pointer` renders a specific schema from the document. It accepts `#`, URI fragment JSON Pointers such as `#/$defs/User`, and JSON Pointers without the fragment marker such as `/$defs/User`. Quote values beginning with `#` in the shell.

### Maximum display depth

`-L N` / `--max-depth N` limits the indentation depth of the rendered tree, counting each root as depth 0. When a node at the limit has children, Puu displays `… [children omitted]` below it instead of silently dropping the children.

### Definition display

`-D MODE` / `--definitions MODE` controls the separate `Definitions` section:

- `all` displays every definition in the document and is the default.
- `referenced` displays definitions transitively reachable through local references from the rendered schema. With `--pointer`, traversal starts at the selected schema.
- `none` omits the section.

References in the rendered schema remain visible in every mode. Definition selection follows schema references independently of `--max-depth`. With `--expand-refs`, references are also expanded inside the `Definitions` section.

### Reference expansion

`-r` / `--expand-refs` displays a locally resolved reference target as a child of the reference line. The reference line retains its own constraints and annotations; the child shows the target schema with an `[expanded from $ref]` marker.

```text
root object
└─ customer -> User [required]
   └─ User object [expanded from $ref]
      ├─ name string [required]
      └─ email string [format: email]
```

Reference chains are expanded transitively. Targets can be definitions, properties, logical branches, or other schemas in the document. Repeated targets are expanded separately in each branch. A reference back to a schema already on the current path is shown with `[expansion stopped: cycle]`.

`--pointer` selects the starting schema; references can still reach targets elsewhere in the document. `--definitions` independently controls the separate section. For a single expanded tree, use:

```console
puu -r --definitions none --max-depth 4 schema.json
puu -r --pointer '#/$defs/User' --definitions none schema.json
```

Expanded target nodes count toward the display depth. Children beyond `--max-depth` are omitted before construction. Reference expansion also has internal limits of display depth 64 and 10,000 constructed expansion nodes across the output, excluding omission markers. Reaching these limits displays `[expansion stopped: depth limit]` or `[expansion stopped: node limit]`.

For locally resolved `$dynamicRef` and `$recursiveRef`, Puu expands the initial target recorded during schema loading. When dynamic resolution could change the target, `[initial target; dynamic scope not evaluated]` makes that limitation explicit. Dynamic scope is not evaluated. External and unresolved references keep their existing markers and are not expanded or fetched.

### Supported scope

Puu supports JSON Schema Draft 2020-12, 2019-09, and Draft 7. It assumes Draft 2020-12 when `$schema` is absent. This is not a complete implementation of each draft; it supports the parts needed to render:

- Boolean schemas, `type`, `enum`, and `const`
- `properties`, `required`, `additionalProperties`, `patternProperties`, and `propertyNames`
- `items`, `prefixItems`, `additionalItems`, and `contains`
- Common string, numeric, object, and array constraints
- `allOf`, `anyOf`, `oneOf`, `not`, and `if` / `then` / `else`
- Property and schema dependencies, `unevaluatedProperties`, and `unevaluatedItems`
- `$id`, `$defs` / `definitions`, `$ref`, anchors, and recursive or dynamic references
- Annotations such as `title`, `description`, `default`, `examples`, `readOnly`, `writeOnly`, and the content-related keywords

References within the input document are resolved for display. External references are shown as external but are not fetched.

Unrecognized keywords are reported as `[uninterpreted: ...]`. Keywords that are recognized but have no effect in their current context are reported as `[ignored: ...]`. With `--verbose`, uninterpreted keyword values and expanded annotation details are included. `format` is displayed as an annotation; instance values are not validated against it.

## License

MIT
