#!/bin/bash

# Looks for `name = "crate_name"`, gets the third word (`"crate_name"`) and removes the quotes.
CRATE_NAME=$(grep name Cargo.toml | awk '{ print $3 }' | tr -d '"')

RUSTFLAGS='--cfg getrandom_backend="wasm_js"' cargo build --release --target wasm32-unknown-unknown

$HOME/.cargo/bin/wasm-bindgen --out-dir serve --target web target/wasm32-unknown-unknown/release/${CRATE_NAME}.wasm
