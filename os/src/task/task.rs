use core::cell::RefMut;

use alloc::{ sync::{Arc, Weak}, vec::Vec};

use crate::{
    config::{kernel_stack_position, TRAP_CONTEXT},
    mm::{MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE},
    sync::UPSafeCell,
    trap::{context::TrapContext, trap_handler},
};

use super::{
    context::TaskContext,
    pid::{KernelStack, PidHandle},
};

/// 任务控制块,内核管理应用的核心数据结构。
///
/// 承担了进程控制块(PCB)的功能
///
/// 负责保存一个任务的状态
/// 由任务状态与任务上下文组成
pub struct TaskControlBlock {
    // 初始化后就不再变化的元数据
    pub pid: PidHandle,
    pub kernel_stack: KernelStack,
    // 运行过程中可能发生变化的元数据
    inner: UPSafeCell<TaskControlBlockInner>,
}

pub struct TaskControlBlockInner {
    // 应用地址空间次高页面上的trap上下文被实际存放的物理页帧号
    pub trap_cx_ppn: PhysPageNum,
    // 应用数据大小
    pub base_size: usize,
    // 将暂停的任务的任务上下文保存在任务控制块中。
    pub task_cx: TaskContext,
    // 任务状态
    pub task_status: TaskStatus,
    // 应用地址空间
    pub memory_set: MemorySet,
    // 当前进程的父进程
    // Weak: 这个智能指针将不会影响父进程的引用计数,子进程在父进程退出后可能仍然存在
    pub parent: Option<Weak<TaskControlBlock>>,
    // 当前进程的所有子进程的任务控制块以 Arc 智能指针的形式保存在一个向量中
    pub children: Vec<Arc<TaskControlBlock>>,
    // 调用 exit 系统调用主动退出或者执行出错由内核终止的时候，它的退出码 exit_code 会被内核保存在它的任务控制块中，
    // 并等待它的父进程通过 waitpid 回收它的资源的同时也收集它的 PID 以及退出码。
    pub exit_code: usize,
}

/// 任务状态
#[derive(Clone, Copy, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Exited,
    Zombie,
}

impl TaskControlBlockInner {
    /// 查找该应用的Trap上下文
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }

    /// 查找应用的地址空间
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }

    fn get_status(&self) -> TaskStatus {
        self.task_status
    }

    pub fn is_zombie(&self) -> bool {
        self.get_status() == TaskStatus::Zombie
    }
}

impl TaskControlBlock {

    /// 创建一个新的进程，目前仅用于内核中创建唯一一个初始进程：initproc
    pub fn new(elf_data: &[u8]) -> Self {
        // 解析传入的 ELF 格式数据构造应用的地址空间 memory_set 并获得其他信息
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);

        // 从地址空间 memory_set 中查多级页表找到应用地址空间中的 Trap 上下文实际被放在哪个物理页帧；
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let task_status = TaskStatus::Ready;

        // 根据传入的应用 ID app_id 调用在 config 子模块中定义的 kernel_stack_position 找到 应用的内核栈预计放在内核地址空间 KERNEL_SPACE 中的哪个位置，
        // 并通过 insert_framed_area 实际将这个逻辑段 加入到内核地址空间中；
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );

        // 用上面的信息来创建并返回任务控制块实例 task_control_block
        let task_control_block = Self {
            task_status,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
        };

        // 查找该应用的 Trap 上下文的内核虚地址。
        let trap_cx = task_control_block.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task_control_block
    }

    /// 实现系统调用，当前进程加载并执行另一个elf格式的可执行文件
    pub fn exec(&self, elf_data: &[u8]) {
        todo!()
    }

    /// 实现fork的系统调用,当前进程fork出一个与之几乎相同的子进程
    pub fn fork(self: &Arc<TaskControlBlock>) -> Arc<TaskControlBlock> {
        todo!()
    }

    /// 获取任务控制块内部的可变引用用于修改任务信息
    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskControlBlockInner> {
        self.inner.exclusive_access()
    }

    /// 获取当前任务的pid
    pub fn getpid(&self) -> usize {
        self.pid.0
    }

}
