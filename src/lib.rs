use rxrust::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet() {
    let o = observable::from_iter(0..10);
    o.clone().subscribe(|_| alert("Hello"));
    o.clone().subscribe(|_| alert("rust-wasm!"));
}
