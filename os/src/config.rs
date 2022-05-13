//! Constants used in rCore

pub const USER_STACK_SIZE: usize = 4096 * 2;
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;
pub const MAX_APP_NUM: usize = 4;
// app运行的起始地址
pub const APP_BASE_ADDRESS: usize = 0x80400000;
// 一个app大小
pub const APP_SIZE_LIMIT: usize = 0x20000;
