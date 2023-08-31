use core::fmt::{Debug, Formatter};

use crate::{config::MEMORY_END, mm::address::PhysAddr, println, sync::up::UPSafeCell};
/// 物理页帧管理器
use alloc::{fmt, vec::Vec};
use lazy_static::lazy_static;

use super::address::PhysPageNum;

type FrameAllocatorImpl = StackFrameAllocator;

lazy_static! {
    pub static ref FRAME_ALLOCATOR: UPSafeCell<FrameAllocatorImpl> =
        unsafe { UPSafeCell::new(FrameAllocatorImpl::new()) };
}

/// 物理页帧管理器的行为
trait FrameAllocator {
    fn new() -> Self;

    // 分配一个物理页
    fn alloc(&mut self) -> Option<PhysPageNum>;

    // 回收一个物理页
    fn dealloc(&mut self, ppn: PhysPageNum);
}

/// 物理页帧管理器
///
/// 根据内存是否被使用过，将其分为两种类型的物理页号进行管理
/// 1. 尚未使用的内存，这部分内存是连续的，因此通过current、end将其进行管理
/// 2. 已使用并被回收的内存，这部分内存是碎片化的，碎片无法很好的重新归入连续空闲内存管理中，因此用Vec将其单独进行管理
#[derive(Debug)]
pub struct StackFrameAllocator {
    // 连续的空闲内存起始页号
    current: usize,
    // 连续的空闲内存终止页号
    end: usize,
    // 已被回收的内存页号
    recycled: Vec<usize>,
}

/// 对物理页号进行封装的物理页帧结构体
///
/// 当开启MMU之后内核对内存地址的访问也变更为虚拟地址，
/// 但在实际运行中需要内核保留以纯软件形式访问实际物理地址的能力
/// 因此内核地址空间中需要存在一个恒等映射到内核数据段之外的可用物理页帧的逻辑段
/// 此时访问地址还是虚拟地址，但是实际等同于物理地址
pub struct FrameTracker {
    pub ppn: PhysPageNum,
}

impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }

    /// 从物理页帧管理器中分配一块空闲内存
    ///
    /// 如果有碎片化的空闲内存，则优先分配碎片化内存
    fn alloc(&mut self) -> Option<PhysPageNum> {
        if let Some(usize) = self.recycled.pop() {
            Some(usize.into())
        } else {
            if self.current == self.end {
                // 内存耗尽
                None
            } else {
                let ppn = self.current;
                self.current -= 1;
                Some(ppn.into())
            }
        }
    }

    /// 让物理页帧管理器回收一块空闲内存
    ///
    /// 先要检查ppn的合法性，然后将其作为碎片化的内存进行回收管理
    fn dealloc(&mut self, ppn: PhysPageNum) {
        if self.recycled.iter().find(|v| ppn.0.eq(v)).is_some() {
            panic!("valid ppn {}", ppn.0)
        }
        self.recycled.push(ppn.0);
    }
}

impl StackFrameAllocator {
    /// 根据提供的一段连续的空闲物理空间的前后页号初始化物理页帧管理器
    fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        println!("[Kernel Debug] current = {}, end = {}", l.0, r.0);
        self.current = l.0;
        self.end = r.0;
    }
}

impl FrameTracker {
    /// 根据物理页号创建物理页帧
    pub fn new(ppn: PhysPageNum) -> Self {
        // 对物理页号指定的内存进行清理
        let array = ppn.get_bytes_array();
        for i in array {
            *i = 0;
        }
        Self { ppn }
    }
}

impl Debug for FrameTracker {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("FrameTracker:PPN={:#x}", self.ppn.0))
    }
}

/// 物理页帧被回收的时候回收物理页号对应的内存
impl Drop for FrameTracker {
    fn drop(&mut self) {
        frame_dealloc(self.ppn)
    }
}

/// 初始化物理页帧管理器
pub fn init_frame_allocator() {
    extern "C" {
        fn ekernel();
    }
    // 初始化的时候要把内核已经占据的内存去除
    FRAME_ALLOCATOR.exclusive_access().init(
        PhysAddr::from(ekernel as usize).ceil(),
        PhysAddr::from(MEMORY_END).floor(),
    );
}

/// 分配物理页帧
///
/// 对外提供的接口
pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR
        .exclusive_access()
        .alloc()
        .map(|ppn| FrameTracker::new(ppn))
}

/// 回收物理页帧
fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
}

#[allow(unused)]
pub fn frame_allocator_test() {
    let mut v: Vec<FrameTracker> = Vec::new();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        let ppn = frame.ppn.0;
        println!("{}", ppn);
        v.push(frame);
    }
    v.clear();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        println!("{:?}", frame.ppn.0);
        v.push(frame);
    }
    drop(v);
    println!("frame_allocator_test passed!");
}
