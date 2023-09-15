mod context;
pub mod manager;
mod pid;
pub mod processor;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::loader::get_app_data_by_name;

pub use processor::{
    current_task, exit_current_and_run_next, run_tasks, schedule, task_current_task,
};

use self::{manager::add_task, task::TaskControlBlock};
use alloc::sync::Arc;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new(TaskControlBlock::new(
        get_app_data_by_name("initproc").unwrap()
    ));
}

pub fn add_initproce() {
    add_task(INITPROC.clone());
}
