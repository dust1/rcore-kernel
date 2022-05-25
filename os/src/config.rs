//! Constants used in rCore

pub const USER_STACK_SIZE: usize = 4096 * 2;
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;
pub const MAX_APP_NUM: usize = 4;
// app运行的起始地址
pub const APP_BASE_ADDRESS: usize = 0x80400000;
// 一个app大小
pub const APP_SIZE_LIMIT: usize = 0x20000;

/// qemu平台的时钟频率，单位为赫兹，也就是一秒内计数器的增量
///
/// 可以看成将1秒分成CLOCK_FREQ份
/// e.g. 后面的CLOCK_FREQ/100等于1s/100=100ms
pub const CLOCK_FREQ: usize = 12500000;
