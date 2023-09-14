use core::cell::RefMut;

use alloc::{
    sync::{Arc, Weak},
    vec::Vec,
};

use crate::{
    config::{kernel_stack_position, TRAP_CONTEXT},
    mm::{MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE},
    sync::UPSafeCell,
    trap::{context::TrapContext, trap_handler},
};

use super::{
    context::TaskContext,
    pid::{pid_alloc, KernelStack, PidHandle},
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
    pub exit_code: i32,
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

        // 分配一个pid和内核栈
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();

        // 用上面的信息来创建并返回任务控制块实例 task_control_block
        let task_control_block = Self {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: user_sp,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                })
            },
        };

        // 查找该应用的 Trap 上下文的内核虚地址,该地址内容为空，只是指针类型为TrapContext
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();

        // 初始化应用地址空间中的TrapContext
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
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        let mut inner = self.inner_exclusive_access();
        // 直接将自身的地址空间替换为elf解析到的地址空间
        // 这会导致原有物理帧被回收
        inner.memory_set = memory_set;
        inner.trap_cx_ppn = trap_cx_ppn;
        let trap_cx = inner.get_trap_cx();

        // 修改TrapContext，将解析得到的应用入口、用户栈位置以及一些内核信息初始化
        // 使得Trap到该任务时能够执行elf所在的代码
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            self.kernel_stack.get_top(),
            trap_handler as usize,
        );
    }

    /// 实现fork的系统调用,当前进程fork出一个与之几乎相同的子进程
    pub fn fork(self: &Arc<TaskControlBlock>) -> Arc<TaskControlBlock> {
        let mut parent_inner = self.inner_exclusive_access();
        // 复制用户地址空间，包含了TrapContext, 并不是通过elf data来创建的
        let memory_set = MemorySet::from_existed_user(&parent_inner.memory_set);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();
        let task_control_block = Arc::new(TaskControlBlock {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    // 子进程和父进程的应用大小保持一致
                    base_size: parent_inner.base_size,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                })
            },
        });

        parent_inner.children.push(Arc::clone(&task_control_block));

        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();
        trap_cx.kernel_sp = kernel_stack_top;

        task_control_block
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
