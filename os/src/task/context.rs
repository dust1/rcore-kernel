/// 任务上下文
/// 只需要保存这三种寄存器，其他寄存器会由调用者保存，或者属于临时寄存器，不需要保存和恢复
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TaskContext {
    // 记录了__switch函数返回后应该跳转到哪里继续执行
    ra: usize,
    // 这个任务所属内核栈的指针
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

    /// 进入用户态
    /// 构造每个任务保存在任务控制块中的任务上下文。
    /// 在 __switch 从它上面恢复并返回之后就会直接跳转到 __restore
    ///
    /// 传入一个内核栈地址
    pub fn goto_restore(kstack_ptr: usize) -> Self {
        extern "C" {
            fn __restore();
        }
        Self {
            ra: __restore as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}
