use wasm_bindgen::prelude::*;

#[wasm_bindgen(
    inline_js = "export function isChromium() { return window.chrome !== null && window.chrome !== undefined; }"
)]
extern "C" {
    fn isChromium() -> bool;
}
pub static ISCHROMIUM: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| isChromium());
