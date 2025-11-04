// src/wallet/src/lib.rs
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen(start)]
pub fn main() {
    log("RoboTorq Wallet WASM loaded!");
}