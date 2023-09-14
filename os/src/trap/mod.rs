// 应用程序通过 ecall 进入到内核状态时，操作系统保存被打断的应用程序的 Trap 上下文；
// 操作系统根据Trap相关的CSR寄存器内容，完成系统调用服务的分发与处理；
// 操作系统完成系统调用服务后，需要恢复被打断的应用程序的Trap 上下文，并通 sret 让应用程序继续执行。

use core::arch::{asm, global_asm};

use riscv::register::{
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
    utvec::TrapMode,
};

use crate::{
    config::{TRAMPOLINE, TRAP_CONTEXT},
    println,
    syscall::syscall,
    task::{processor::{
        current_task, current_trap_cx, current_user_token, suspend_current_and_run_next,
    }, exit_current_and_run_next},
    timer::set_next_trigger,
};

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

fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernl as usize, TrapMode::Direct);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE, TrapMode::Direct);
    }
}

#[no_mangle]
pub fn trap_from_kernl() {
    panic!("a trap from kernel!");
}

#[no_mangle]
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_cx_ptr = TRAP_CONTEXT;
    let user_satp = current_user_token();
    extern "C" {
        fn __alltraps();
        fn __restore();
    }
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,
            in("a1") user_satp,
            options(noreturn)
        );
    }
}

/// 在S模式下被调用
///
/// 当S/U模式下发起trap的时候会调用该函数进行分发和处理
#[no_mangle]
pub fn trap_handler() -> ! {
    set_kernel_trap_entry();
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        // 来自U特权级的ecall调用(系统调用)
        Trap::Exception(Exception::UserEnvCall) => {
            let mut cx = current_trap_cx();
            // U特权发起系统调用后sepc寄存器保存的是ecall指令地址,我们希望trap返回后应用程序控制流从ecall的下一条指令开始执行
            // 因此我们增加sepc的长度,4就是ecall指令的码长(4字节)
            cx.sepc += 4;
            // 从a7(x17)寄存器读取syscall的ID
            // 从a0~a2(x10~x12)寄存器读取本次syscall的参数
            let result = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;

            // 原来的cx会被回收，需要重新获取
            cx = current_trap_cx();
            cx.x[10] = result as usize;
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault)
        | Trap::Exception(Exception::InstructionFault)
        | Trap::Exception(Exception::InstructionPageFault) => {
            println!(
                "[kernel] {:?} in application, bad addr = {:#x}, bad instruction = {:#x}, core dumped.",
                scause.cause(),
                stval,
                current_trap_cx().sepc,
            );
            exit_current_and_run_next(-2);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, core dumped.");
            exit_current_and_run_next(-3);
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            println!("[kernel] SupervisorTimer!!!");
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
    };
    trap_return()
}
