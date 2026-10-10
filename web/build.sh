#!/bin/sh
# Build the web demo into web/www/pkg. Needs the wasm32 target and a
# wasm-bindgen CLI of the same version as the wasm-bindgen crate:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <version in Cargo.lock>
# Then serve web/www, for example: python3 -m http.server -d web/www
set -e
cd "$(dirname "$0")/.."
cargo build --profile web --target wasm32-unknown-unknown -p glowdots-web
wasm-bindgen --target web --no-typescript --out-dir web/www/pkg target/wasm32-unknown-unknown/web/glowdots_web.wasm
if command -v wasm-opt >/dev/null; then
    wasm-opt -Os --enable-bulk-memory --enable-nontrapping-float-to-int -o web/www/pkg/glowdots_web_bg.wasm web/www/pkg/glowdots_web_bg.wasm
fi
ls -l web/www/pkg
