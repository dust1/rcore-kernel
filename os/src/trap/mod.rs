use core::arch::global_asm;

use riscv::register::{
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
    utvec::TrapMode,
};

use crate::{
    println, syscall::syscall, task::suspend_current_and_run_next, timer::set_next_trigger,
};

use self::context::TrapContext;

pub mod context;

// trap 的上下文保存与恢复
// __alltraps的实现
global_asm!(include_str!("trap.S"));

pub fn init() {
    extern "C" {
        // 引入外部符号__alltraps
        fn __alltraps();
    }
    unsafe {
        // 将stvec设置为Direct模式,并指向__alltraps的地址
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}

/// 设置了sie.stie使得S特权级时钟终端不会被屏蔽
pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

/// 在S模式下被调用
#[no_mangle]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        // 来自U特权级的ecall调用(系统调用)
        Trap::Exception(Exception::UserEnvCall) => {
            // U特权发起系统调用后sepc寄存器保存的是ecall指令地址,我们希望trap返回后应用程序控制流从ecall的下一条指令开始执行
            // 因此我们增加sepc的长度,4就是ecall指令的码长(4字节)
            cx.sepc += 4;
            // 从a7(x17)寄存器读取syscall的ID
            // 从a0~a2(x10~x12)寄存器读取本次syscall的参数
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            println!("[kernel] PageFault in application, kernel killed it.");
            panic!("[kernel] Cannot continue")
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, kernel killed it.");
            panic!("[kernel] Cannot continue")
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            // println!("[kernel] SupervisorTimer!!!");
            set_next_trigger();
            suspend_current_and_run_next();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }

    cx
}
