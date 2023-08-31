use self::memory_set::KERNEL_SPACE;

pub mod heap_allocator;

pub mod address;

pub mod page_table;

pub mod frame_allocator;

pub mod memory_set;

pub use memory_set::remap_test;

/// 内存结构初始化
pub fn init() {
    // 初始化全局内存动态分配器
    heap_allocator::init_heap();
    // 初始化物理页帧管理器
    frame_allocator::init_frame_allocator();
    // 创建内核地址空间
    KERNEL_SPACE.exclusive_access().activate();
}
