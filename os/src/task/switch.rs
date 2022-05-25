use core::arch::global_asm;

use super::context::TaskContext;

global_asm!(include_str!("switch.S"));

/// 将汇编代码中的全局符号__switch解释为一个RUST函数
extern "C" {
    pub fn __switch(current_task_cx_ptr: *mut TaskContext, next_task_cx_ptr: *const TaskContext);
}
