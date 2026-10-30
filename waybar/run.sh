#!/usr/bin/env bash
REAL_DIR="$(cd "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")" && pwd)"
BINARY="$REAL_DIR/../target/release/waybar_claude"
[ -x "$BINARY" ] || BINARY="waybar_claude"
exec "$BINARY" "$@"
