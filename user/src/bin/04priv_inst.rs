#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use core::arch::asm;

#[no_mangle]
fn main() -> i32 {
    println!("Try to exectue privileged instruction in U mode");
    println!("Kernel should kill this application!");
    // 尝试在用户态执行sret指令
    unsafe {
        asm!("sret");
    }
    0
}