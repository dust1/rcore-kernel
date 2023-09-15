use alloc::sync::Arc;
use lazy_static::lazy_static;

use crate::{sync::UPSafeCell, task::INITPROC, trap::context::TrapContext};

use super::{
    context::TaskContext,
    manager::{add_task, fetch_task},
    switch::__switch,
    task::{TaskControlBlock, TaskStatus},
};

/// 处理器管理结构负责维护从任务管理器中分离出来的CPU状态
pub struct Processor {
    // 当前处理器上正在执行的任务
    current: Option<Arc<TaskControlBlock>>,
    // 当前处理器上的idle控制流的上下文
    idle_task_cx: TaskContext,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }

    /// 取出当前正在执行的任务
    pub fn task_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }

    /// 返回当前正在执行的任务的一份拷贝
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(Arc::clone)
    }

    /// 获取当前idle控制流的task_cx_ptr
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }
}

impl Default for Processor {
    fn default() -> Self {
        Self::new()
    }
}

/// 取出当前正在执行的任务
pub fn task_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().task_current()
}

/// 返回当前正在执行的任务的一份拷贝
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().current()
}

/// 获取当前正在执行的任务的地址空间
pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    let token = task.inner_exclusive_access().get_user_token();
    token
}

// 退出当前任务并执行下一个任务
pub fn exit_current_and_run_next(exit_code: i32) {
    let task = task_current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    inner.task_status = TaskStatus::Zombie;
    inner.exit_code = exit_code;

    {
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        // 将要结束的进程的子进程挂靠到初始进程中
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }

    inner.children.clear();
    inner.memory_set.recycle_data_page();
    drop(inner);
    drop(task);

    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

/// 暂停当前的应用并切换到下一个应用
pub fn suspend_current_and_run_next() {
    // 获取当前正在运行的任务
    let task = task_current_task().unwrap();

    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;

    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);

    // 放入进程管理队列末尾
    add_task(task);
    schedule(task_cx_ptr);
}

/// 获取当前正在执行的任务的应用的Trap上下文
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_trap_cx()
}

/// 运行任务
pub fn run_tasks() {
    loop {
        let mut processor = PROCESSOR.exclusive_access();
        if let Some(task) = fetch_task() {
            let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();

            // 从任务管理器中获取下一个任务的
            let mut task_inner = task.inner_exclusive_access();
            let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
            task_inner.task_status = TaskStatus::Running;

            drop(task_inner);
            // 将任务放置到当前正在执行的任务中，
            // 因此对于TaskControlBlock来说引用计数器都为1，
            // 要么在任务管理器，要么在处理器管理结构中
            processor.current = Some(task);

            drop(processor);

            // 在这里会对task的TaskContext进行访问，因此上面需要drop掉相关对象释放资源
            unsafe { __switch(idle_task_cx_ptr, next_task_cx_ptr) }
        }
    }
}

/// 切换任务上下文
///
/// switched_task_cx_ptr: 带切换出去的任务上下文
pub fn schedule(switchded_task_cx_ptr: *mut TaskContext) {
    let mut processor = PROCESSOR.exclusive_access();
    let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
    drop(processor);
    unsafe {
        __switch(switchded_task_cx_ptr, idle_task_cx_ptr);
    }
}

lazy_static! {
    /// 单核情况下只需要创建一个Processor实例
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe {
        UPSafeCell::new(Processor::new())
    };
}
