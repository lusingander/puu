#!/usr/bin/env bash

set -euo pipefail

readonly ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ASSET_DIR="$ROOT_DIR/docs/assets/options"
readonly FREEZE_VERSION="v0.2.2"

temporary_dir="$(mktemp -d "${TMPDIR:-/tmp}/puu-option-images.XXXXXX")"
readonly temporary_dir
readonly temporary_asset_dir="$temporary_dir/assets"
readonly freeze_bin="$temporary_dir/bin/freeze"

mkdir -p "$temporary_asset_dir" "$(dirname "$freeze_bin")"

render_image() {
  local image_name="$1"
  local color="$2"
  local input_path="$3"
  shift 3

  "$ROOT_DIR/target/debug/puu" "--color=$color" "$@" "$ROOT_DIR/$input_path" \
    | "$freeze_bin" \
      --config "$ROOT_DIR/docs/freeze.json" \
      --output "$temporary_asset_dir/$image_name.png"
}

render_all() {
  # Keep each option's examples together so they can be reviewed independently.
  render_image \
    "draft-2019-09" \
    "always" \
    "docs/examples/options/draft.json" \
    --draft 2019-09
  render_image \
    "draft-7" \
    "always" \
    "docs/examples/options/draft.json" \
    --draft 7

  render_image \
    "pointer-user" \
    "always" \
    "docs/examples/options/pointer.json" \
    --pointer '#/$defs/User'
}

(
  cd "$ROOT_DIR"
  cargo build --locked
)

GOBIN="$(dirname "$freeze_bin")" \
  go install "github.com/charmbracelet/freeze@$FREEZE_VERSION"

render_all

mkdir -p "$ASSET_DIR"
find "$ASSET_DIR" -type f -name '*.png' -delete
cp "$temporary_asset_dir"/*.png "$ASSET_DIR"/
chmod 0644 "$ASSET_DIR"/*.png

printf 'Generated %s option images in %s.\n' \
  "$(find "$ASSET_DIR" -type f -name '*.png' | wc -l | tr -d ' ')" \
  "$ASSET_DIR"
