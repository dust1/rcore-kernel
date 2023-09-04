//! Memory management implementation
//!
//! SV39 page-based virtual-memory architecture for RV64 systems, and
//! everything about memory management, like frame allocator, page table,
//! map area and memory set, is implemented here.
//!
//! Every task or process has a memory_set to control its virtual memory.

mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
pub use frame_allocator::{frame_alloc, FrameTracker};
pub use memory_set::remap_test;
pub use memory_set::{MapPermission, MemorySet, KERNEL_SPACE};
pub use page_table::{translated_byte_buffer, PageTableEntry};

/// initiate heap allocator, frame allocator and kernel space
pub fn init() {
    // 初始化全局内存动态分配器
    heap_allocator::init_heap();
    // 初始化物理页帧管理器
    frame_allocator::init_frame_allocator();
    // 创建内核地址空间
    KERNEL_SPACE.exclusive_access().activate();
}
