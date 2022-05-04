#![no_std]
#![no_main]
#![feature(panic_info_message)]

mod lang_items;
mod sbi;

#[macro_use]
mod console;

use core::{arch::global_asm};

global_asm!(include_str!("entry.asm"));

#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    println!("Hello World!!");
    panic!("Shutdown machine!")
}

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    // 找到全局符号sbss,ebss(它们由连接脚本linker.ld给出)
    // 分别指向要被清零的.bss起始段和终止地址
    (sbss as usize..ebss as usize).for_each(|a| {
        // 将这两个地址之间的内存清零
        unsafe {
            (a as *mut u8).write_volatile(0)
        }
    });
}
