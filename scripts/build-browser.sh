#!/usr/bin/env bash
set -euo pipefail
TODO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$TODO_ROOT"
export FERROUI_BROWSER_WORKSPACE="$TODO_ROOT"
# Bound LLVM memory on ordinary runners. Thin LTO still optimizes across codegen units.
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
export CARGO_PROFILE_BROWSER_CODEGEN_UNITS="${CARGO_PROFILE_BROWSER_CODEGEN_UNITS:-16}"
TODO_SITE="target/browser/todo-browser"
TODO_ARGUMENTS=("$@")
for ((TODO_INDEX=0; TODO_INDEX<${#TODO_ARGUMENTS[@]}; TODO_INDEX++)); do
  if [ "${TODO_ARGUMENTS[$TODO_INDEX]}" = "--out" ]; then TODO_SITE="${TODO_ARGUMENTS[$((TODO_INDEX+1))]}"; fi
done
FerroUI/scripts/build-browser.sh todo-browser "$@"
# PST parsing runs in a dedicated worker without loading the UI/Skia module.
rustup target add wasm32-unknown-unknown
cargo build --locked --release --target wasm32-unknown-unknown -p todo-interop-wasm
wasm-bindgen target/wasm32-unknown-unknown/release/todo_interop_wasm.wasm \
  --target web --out-dir "$TODO_SITE/interop" --out-name todo_interop_wasm
node scripts/verify-pst-wasm.mjs "$TODO_SITE/interop"
node scripts/configure-microsoft.mjs "$TODO_SITE"
node scripts/prepare-site.mjs "$TODO_SITE"
