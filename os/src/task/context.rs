use crate::trap::trap_return;

/// 任务上下文
/// 只需要保存这三种寄存器，其他寄存器会由调用者保存，或者属于临时寄存器，不需要保存和恢复
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TaskContext {
    // 记录了__switch函数返回后应该跳转到哪里继续执行
    ra: usize,
    sp: usize,
    s: [usize; 12],
}

impl TaskContext {
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }

    pub fn goto_trap_return(kstack_ptr: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}
