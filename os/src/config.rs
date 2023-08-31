//! Constants used in rCore

pub const USER_STACK_SIZE: usize = 4096 * 2;
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;
pub const MAX_APP_NUM: usize = 4;
// app运行的起始地址
pub const APP_BASE_ADDRESS: usize = 0x80400000;
// 一个app大小
pub const APP_SIZE_LIMIT: usize = 0x20000;

pub use crate::board::CLOCK_FREQ;

/// 一个内存页的比特大小
///
/// 页内偏移的位宽
pub const PAGE_SIZE_BITS: usize = 12;

/// 一个内存页面的大小：4KB
pub const PAGE_SIZE: usize = 4096;

/// 内存大小
pub const MEMORY_END: usize = 0x80800000;

pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;

pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;

pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}