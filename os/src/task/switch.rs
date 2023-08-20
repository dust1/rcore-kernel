use core::arch::global_asm;

use super::context::TaskContext;

global_asm!(include_str!("switch.S"));

// 将汇编代码中的全局符号__switch解释为一个RUST函数
extern "C" {

    /// 交换两个任务上下文的寄存器
    ///
    /// current_task_cx_ptr: a0寄存器，要切出保存的任务上下文
    /// next_task_cx_ptr: a1寄存器，要切入的下一个执行任务的上下文
    pub fn __switch(current_task_cx_ptr: *mut TaskContext, next_task_cx_ptr: *const TaskContext);
}
