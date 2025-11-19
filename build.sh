#!/bin/bash

# Looks for `name = "crate_name"`, gets the third word (`"crate_name"`) and removes the quotes.
CRATE_NAME=$(grep name Cargo.toml | awk '{ print $3 }' | tr -d '"')

cargo build --release --target wasm32-unknown-unknown

$HOME/.cargo/bin/wasm-bindgen --out-dir serve --target web target/wasm32-unknown-unknown/release/${CRATE_NAME}.wasm

$HOME/.cargo/bin/wasm-opt serve/${CRATE_NAME}_bg.wasm -o serve/${CRATE_NAME}_bg.wasm -Oz -all --fast-math

cp -r assets/ serve/assets/