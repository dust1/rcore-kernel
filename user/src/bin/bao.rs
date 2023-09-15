#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

extern crate alloc;

use alloc::vec::Vec;
use user_lib::get_time;

#[no_mangle]
pub fn main() -> i32 {
    let time = get_time();
    let count = time % 10;
    let mut wolds = Vec::new();
    wolds.push("I love a ke bao!");
    wolds.push("kiss kiss a ke bao!");
    wolds.push("mua a ke bao!");
    for _ in 0..count {
        let t = get_time() % 3;
        println!("{}", &wolds[t as usize]);
    }
    0
}
