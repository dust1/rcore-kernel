mod context;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::{
    loader::{get_app_data, get_num_app},
    println,
    sbi::shutdown,
    sync::up::UPSafeCell,
    task::{context::TaskContext, task::TaskStatus},
    trap::context::TrapContext,
};

use self::{switch::__switch, task::TaskControlBlock};
use alloc::vec::Vec;
use lazy_static::lazy_static;

/// 任务管理器
///
pub struct TaskManager {
    // 所有任务数量
    num_app: usize,
    // 任务管理器内部,通过UPSafeCell来约束可变引用数量
    inner: UPSafeCell<TaskManagerInner>,
}

/// 任务管理器内部
pub struct TaskManagerInner {
    // 任务列表
    tasks: Vec<TaskControlBlock>,
    // 当前正在运行的任务id
    current_task: usize,
}

// 找到 link_app.S 中提供的符号 _num_app ，并从这里开始解析出应用数量以及各个应用的起始地址。
lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        println!("[kernel] init task mamager, total app size: {}", num_app);
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
        }

        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}

impl TaskManager {
    /// 运行第一个任务
    pub fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;

        drop(inner);
        // 创建一个空的上下文作为初始控制流
        let mut _unused = TaskContext::zero_init();
        unsafe {
            __switch(&mut _unused as *mut _, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!")
    }

    /// 运行下一个任务
    pub fn run_next_task(&self) {
        if let Some(app_id) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();

            let current = inner.current_task;

            // 将下一个任务设置为Running
            inner.tasks[app_id].task_status = TaskStatus::Running;
            inner.current_task = app_id;

            // 获取任务进行切换所需的两个任务的上下文对象
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[app_id].task_cx as *const TaskContext;

            drop(inner);

            // 任务寄存器调换
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }

            // 返回用户态
        } else {
            // 当所有任务结束的时候,并不会调用__switch,这会导致这个任务对应的调用栈里的栈空间无法再使用
            println!("[Kernel] all task completed!");
            shutdown(false);
        }
    }

    /// 找到下一个任务并且返回任务的app_id
    ///
    /// 我们只需要找到任务列表中下一个处于Ready状态的任务,不在乎是否到列表结尾
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        // 循环执行
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// 将任务状态从Running到Ready
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        // 获取当前正在运行的任务
        let current = inner.current_task;
        // 将当前正在与运行的任务状态修改为Ready
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    /// 将任务状态从Running到Exited
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        // 当前运行的任务id
        let current = inner.current_task;

        inner.tasks[current].task_status = TaskStatus::Exited;
        println!("[kernel] Task PID: {},", current,);
    }

    /// 获得当前正在执行的应用的地址空间的 token
    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].get_user_token()
    }

    /// 获得当前正在执行的应用的Trap上下文
    fn get_current_cx(&self) -> &mut TrapContext {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].get_trap_cx()
    }
}

pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_cx()
}

pub fn run_first_app() {
    TASK_MANAGER.run_first_task();
}

pub fn run_next_task() {
    TASK_MANAGER.run_next_task()
}

pub fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended()
}

pub fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited()
}

/// 暂停当前的应用并切换到下一个应用
pub fn suspend_current_and_run_next() {
    // 将当前的任务从Running修改为Ready
    mark_current_suspended();
    // 运行下一个任务
    run_next_task();
}

/// 退出当前的应用并切换到下个应用
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}
