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
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }

    /// entry - 应用程序入口点
    pub fn app_init_context(entry: usize, sp: usize) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spp(SPP::User);
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,
        };
        cx.set_sp(sp);
        cx
    }
}
