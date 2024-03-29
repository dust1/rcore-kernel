#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

extern crate alloc;

#[macro_use]
extern crate bitflags;

#[path = "boards/qemu.rs"]
mod board;

#[macro_use]
mod console;
mod config;
mod lang_items;
mod loader;
mod mm;
mod sbi;
mod sync;
pub mod syscall;
pub mod task;
mod timer;
pub mod trap;

use core::arch::global_asm;

// 嵌入汇编代码,首先执行这段汇编代码
global_asm!(include_str!("entry.asm"));
// 寻找应用程序并连接
global_asm!(include_str!("link_app.S"));

#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    println!("[kernel] clear bss was ok...");
    mm::init();
    println!("[kernel] init memory was ok...");
    mm::remap_test();
    task::add_initproce();
    println!("[kernel] memory test all pass...");
    // S模式运行
    trap::init();
    // 设置S特权级的时钟中断不会被屏蔽
    trap::enable_timer_interrupt();
    // 设置第一个10ms计时器
    timer::set_next_trigger();
    loader::list_apps();
    task::run_tasks();
    panic!("Unreachable in rust_main!")
}

// 对 .bss 段的清零
// 在使用任何被分配到 .bss 段的全局变量之前我们需要确保 .bss 段已被清零。
fn clear_bss() {
    extern "C" {
        // .bss段的起始地址
        fn sbss();
        // .bss段的终止地址
        fn ebss();
    }

    // 找到全局符号sbss,ebss(它们由连接脚本linker.ld给出)
    // 分别指向要被清零的.bss起始段和终止地址
    (sbss as usize..ebss as usize).for_each(|a| {
        // 将这两个地址之间的内存清零
        unsafe { (a as *mut u8).write_volatile(0) }
    });
}
