use riscv::register::sstatus::{self, Sstatus, SPP};

/// Trap上下文
/// 在Trap发生时保存物理资源内容
#[repr(C)]
pub struct TrapContext {
    /// 通用寄存器x0~x31
    pub x: [usize; 32],
    /// 下面两个参数会随着trap的嵌套而覆盖，因此需要专门保存下来
    /// sstatus
    pub sstatus: Sstatus,
    /// sepc
    pub sepc: usize,
    /// 内核地址空间的token, 即内核页表的起始物理地址
    pub kernel_satp: usize,
    /// 当前应用在内核地址空间中的内核栈栈顶的虚拟地址
    pub kernel_sp: usize,
    /// 内核中trap handler入口的虚拟地址
    pub trap_handler: usize,
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }

    /// entry - 应用程序在内存中的起始地址
    /// sp: 在用户栈中该应用程序所在的栈的起始地址
    pub fn app_init_context(
        entry: usize,
        sp: usize,
        kernel_satp: usize,
        kernel_sp: usize,
        trap_handler: usize,
    ) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spp(SPP::User);
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,
            kernel_satp,
            kernel_sp,
            trap_handler,
        };
        cx.set_sp(sp);
        cx
    }
}
