#!/usr/bin/env bash
# Build the Tiller UI (Leptos WASM) then the ycp binary.
# Order matters once /next flips to rust-embed at P5; today it just keeps
# rust/ui/dist fresh for the /next disk-serving route.
set -euo pipefail
cd "$(dirname "$0")/../ui"
trunk build --release
cd ..
cargo build --release
