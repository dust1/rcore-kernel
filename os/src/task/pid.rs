use alloc::vec::Vec;
use lazy_static::lazy_static;

use crate::{
    config::kernel_stack_position,
    mm::{MapPermission, VirtAddr, KERNEL_SPACE},
    sync::UPSafeCell,
};

lazy_static! {
    static ref PID_ALLOCATOR: UPSafeCell<PidAllocator> =
        unsafe { UPSafeCell::new(PidAllocator::new()) };
}

/// 进程标识符
pub struct PidHandle(pub usize);

/// 和物理页帧分配器一样概念的进程标识符分配器，用于分配pid
struct PidAllocator {
    current: usize,
    recycled: Vec<usize>,
}

/// 进程标识符，在内核栈中保存进程的pid
pub struct KernelStack {
    pid: usize,
}

impl KernelStack {
    /// 从一个已经分配的进程标识符中生成一个对应的内核栈
    pub fn new(pid_handle: &PidHandle) -> Self {
        let pid = pid_handle.0;
        // 根据标识符计算内核栈在内核地址空间中的位置
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(pid);
        // 将逻辑段插入到内核地址空间中根据pid创建该任务应该所在的地址空间范围[kernel_stack_bottom, kernel_stack_top)
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        KernelStack { pid: pid_handle.0 }
    }

    /// 将一个类型为T的变量压入内核栈顶并返回裸指针
    pub fn push_on_top<T>(&self, value: T) -> *mut T
    where
        T: Sized,
    {
        let kernel_stack_top = self.get_top();
        let ptr_mut = (kernel_stack_top - core::mem::size_of::<T>()) as *mut T;
        unsafe {
            *ptr_mut = value;
        }
        ptr_mut
    }

    /// 获取当前内核栈顶在内核地址空间中的地址
    pub fn get_top(&self) -> usize {
        let (_, kernel_stack_top) = kernel_stack_position(self.pid);
        kernel_stack_top
    }
}

impl Drop for KernelStack {
    /// 一旦生命周期结束则在内核地址空间中将对应的逻辑段删除
    fn drop(&mut self) {
        let (kernel_stack_bottom, _) = kernel_stack_position(self.pid);
        let kernel_stack_bottom_va: VirtAddr = kernel_stack_bottom.into();
        KERNEL_SPACE
            .exclusive_access()
            .remove_area_with_start_vpn(kernel_stack_bottom_va.into());
    }
}

impl PidAllocator {
    pub fn new() -> Self {
        Self {
            current: 0,
            recycled: Vec::new(),
        }
    }

    pub fn alloc(&mut self) -> PidHandle {
        if let Some(pid) = self.recycled.pop() {
            PidHandle(pid)
        } else {
            self.current += 1;
            PidHandle(self.current - 1)
        }
    }

    pub fn dealloc(&mut self, pid: usize) {
        assert!(pid < self.current);
        if self.recycled.iter().any(|p| pid.eq(&p)) {
            println!("[Kernel] pid: {} was dealloc!", pid);
        } else {
            self.recycled.push(pid);
        }
    }
}

impl Drop for PidHandle {
    fn drop(&mut self) {
        pid_dealloc(self.0)
    }
}

pub fn pid_alloc() -> PidHandle {
    PID_ALLOCATOR.exclusive_access().alloc()
}

fn pid_dealloc(pid: usize) {
    PID_ALLOCATOR.exclusive_access().dealloc(pid);
}
