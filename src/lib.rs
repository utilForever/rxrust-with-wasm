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
    // "0 2 4 6 8 1 3 5 7 9" will be printed
    even.merge(odd).subscribe(|v| log!("{} ", v, ));

    log!("example_basic() - end");
}

#[wasm_bindgen]
pub fn example_first() {
    log!("example_first() - start");

    // create an numbers stream
    let numbers = observable::from_iter(1..=5);
    let first = numbers.clone().first();

    // "1" will be printed
    first.subscribe(|v| log!("{} ", v, ));

    log!("example_first() - end");
}

#[wasm_bindgen]
pub fn example_last() {
    log!("example_last() - start");

    // create an numbers stream
    let numbers = observable::from_iter(1..=5);
    let last = numbers.clone().last();

    // "5" will be printed
    last.subscribe(|v| log!("{} ", v, ));

    log!("example_last() - end");
}

#[wasm_bindgen]
pub fn example_skip() {
    log!("example_skip() - start");

    // create an numbers stream
    let numbers = observable::from_iter(1..=10);
    let skip = numbers.clone().skip(5);

    // "6, 7, 8, 9, 10" will be printed
    skip.subscribe(|v| log!("{} ", v, ));

    log!("example_skip() - end");
}
