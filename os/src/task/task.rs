use super::context::TaskContext;

/// 任务控制块
///
/// 负责保存一个任务的状态
/// 由任务状态与任务上下文组成
#[derive(Clone, Copy)]
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    // 开始时间
    pub start_time: usize,
    // 应用完成时间
    pub app_end_time: usize,
    // 内核完成时间
    pub kernel_end_time: usize,
}

#[derive(Clone, Copy, PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}
