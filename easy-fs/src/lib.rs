#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

extern crate alloc;

mod block_dev;
mod block_cache;

/// 块的大小，和磁盘扇区大小一致,都是512字节
pub const BLOCK_SZ: usize = 512;