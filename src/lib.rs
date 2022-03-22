extern crate web_sys;

use rxrust::prelude::*;
use wasm_bindgen::prelude::*;

// A macro to provide 'println!(..)'-style syntax for 'console.log' logging.
#[allow(unused_macros)]
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into())
    }
}

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet() {
    let o = observable::from_iter(0..10);
    o.clone().subscribe(|_| log!("Hello"));
    o.clone().subscribe(|_| log!("rust-wasm!"));
}
