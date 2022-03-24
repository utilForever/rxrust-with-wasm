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
pub fn example_basic() {
    log!("example_basic() - start");

    let numbers = observable::from_iter(0..10);
    // create an even stream by filter
    let even = numbers.clone().filter(|v| v % 2 == 0);
    // create an odd stream by filter
    let odd = numbers.clone().filter(|v| v % 2 != 0);
    
    // merge odd and even stream again
    even.merge(odd).subscribe(|v| log!("{} ", v, ));
    // "0 2 4 6 8 1 3 5 7 9" will be printed.

    log!("example_basic() - end");
}
