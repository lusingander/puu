# CLI Options

This guide shows how each output-related Puu option changes the rendered tree. The images are generated from the real CLI with [Freeze](https://github.com/charmbracelet/freeze).

Run the following command from the repository root to update the images:

```sh
./scripts/generate-option-images.sh
```

## Contents

- [Draft override](#draft-override)
- [JSON Pointer selection](#json-pointer-selection)

## Draft override

`-d DRAFT` / `--draft DRAFT` interprets the entire input as the selected JSON Schema draft, overriding its `$schema` declaration when present. The supported values are `2020-12`, `2019-09`, and `7`. Without this option, Puu uses the declared draft or assumes Draft 2020-12 when `$schema` is absent.

The example contains dependency keywords from both the modern drafts and Draft 7:

```json
{
  "type": "object",
  "dependencies": {
    "legacy": ["oldTarget"]
  },
  "dependentRequired": {
    "modern": ["newTarget"]
  }
}
```

[Example input](examples/options/draft.json)

With Draft 2019-09, the modern dependency keywords are interpreted and `dependencies` is left uninterpreted:

```console
puu --draft 2019-09 docs/examples/options/draft.json
```

![Output interpreted as Draft 2019-09](assets/options/draft-2019-09.png)

With Draft 7, `dependencies` is interpreted and the newer keywords are left uninterpreted:

```console
puu --draft 7 docs/examples/options/draft.json
```

![Output interpreted as Draft 7](assets/options/draft-7.png)

The override applies to embedded resources as well as the document root. It can therefore make an input invalid when a keyword has a different shape in the selected draft.

## JSON Pointer selection

`-p POINTER` / `--pointer POINTER` renders a schema at a particular location instead of starting at the document root. It accepts `#`, URI fragment JSON Pointers such as `#/$defs/User`, and JSON Pointers without the fragment marker such as `/$defs/User`.

```json
{
  "type": "object",
  "properties": {
    "orderId": { "type": "string" }
  },
  "$defs": {
    "User": {
      "type": "object",
      "required": ["name"],
      "properties": {
        "name": { "type": "string" },
        "email": { "type": "string", "format": "email" }
      }
    }
  }
}
```

[Example input](examples/options/pointer.json)

```console
puu --pointer '#/$defs/User' docs/examples/options/pointer.json
```

![Output selected at the User definition](assets/options/pointer-user.png)

The selected node is labeled with its document location. Definitions are still displayed according to `--definitions`; the default `all` mode explains why `User` also appears in the separate `Definitions` section above. With `--definitions referenced`, reference traversal starts at the selected schema.

Quote pointers beginning with `#` so that the shell does not treat the value as a comment. A pointer must identify a schema rather than an arbitrary JSON value such as a `$ref` string.
