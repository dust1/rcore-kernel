use core::borrow::BorrowMut;

use crate::{
    config::MAX_APP_NUM,
    loader::{get_num_app, init_app_cx},
    sync::up::UPSafeCell,
    task::{context::TaskContext, task::TaskStatus},
};

use self::{switch::__switch, task::TaskControlBlock};
use lazy_static::lazy_static;

mod context;
mod switch;
mod task;

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
    tasks: [TaskControlBlock; MAX_APP_NUM],
    // 当前正在运行的任务id
    current_task: usize,
}

// 初始化TSAK_MANAGER,这些数据本质上都被保存在.data段中
lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        let mut tasks = [TaskControlBlock {
            task_status: TaskStatus::UnInit,
            task_cx: TaskContext::zero_init(),
        }; MAX_APP_NUM];
        for (i, task) in tasks.iter_mut().enumerate() {
            task.task_cx = TaskContext::goto_restore(init_app_cx(i));
            task.task_status = TaskStatus::Ready;
        }

        let task_manager_inner = TaskManagerInner {
            tasks,
            current_task: 0,
        };
        let task_manager = TaskManager {
            num_app,
            inner: unsafe { UPSafeCell::new(task_manager_inner) },
        };

        task_manager
    };
}

impl TaskManager {
    pub fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        drop(inner);
        // 创建一个空的上下文作为初始控制流
        let mut _unused = TaskContext::zero_init();
        unsafe {
            __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!")
    }

    pub fn run_next_task(&self) {
        if let Some(app_id) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[app_id].task_status = TaskStatus::Running;
            inner.current_task = app_id;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[app_id].task_cx as *const TaskContext;
            drop(inner);

            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
        } else {
            // 当所有任务结束的时候,并不会调用__switch,这会导致这个任务对应的调用栈里的栈空间无法再使用
            panic!("All application completed!")
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
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    /// 将任务状态从Running到Exited
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }
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

pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}
