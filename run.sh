#!/usr/bin/bash

RUSTFLAGS='--cfg getrandom_backend="wasm_js"' cargo run --target wasm32-unknown-unknown 