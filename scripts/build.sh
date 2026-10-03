#!/bin/sh
# Build the keynote binary locally (editor window included).
# Needs Rust 1.90 or newer. On Linux the window also needs WebKit:
#   sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev \
#     libayatana-appindicator3-dev librsvg2-dev patchelf
set -eu
cd "$(dirname "$0")/.."
if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is not installed. Install Rust from https://rustup.rs" >&2
  exit 1
fi
cargo build --release --features native-view
target="${CARGO_TARGET_DIR:-$(pwd)/target}"
bin="$target/release/keynote"
echo "built $bin"
