use core::arch::asm;

use lazy_static::*;

use crate::{
    config::{APP_BASE_ADDRESS, APP_SIZE_LIMIT},
    loader::load_apps,
    println,
    sync::up::UPSafeCell,
    trap::context::TrapContext,
};

const MAX_APP_NUM: usize = 16;

/// 两个常数分别指出内核栈和用户栈的大小为8KB
/// 根据程序的布局，这两个常数以全局变量的形式实例化在.bss段中
/// 用户栈大小
const USER_STACK_SIZE: usize = 4096 * 2;
/// 内核栈大小
const KERNEL_STACK_SIZE: usize = 4096 * 2;

struct AppManager {
    // app个数
    num_app: usize,
    // 当前执行的是第几个应用
    current_app: usize,
}

/// 内核栈
#[repr(align(4096))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

/// 用户栈
#[repr(align(4096))]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

static KERNEL_STACK: KernelStack = KernelStack {
    data: [0u8; KERNEL_STACK_SIZE],
};
static USER_STACK: UserStack = UserStack {
    data: [0u8; USER_STACK_SIZE],
};

impl KernelStack {
    // 获取栈的栈顶地址
    fn get_sp(&self) -> usize {
        // 由于在RISC-V中栈是向低地址增长的，我们只需要返回data数组的结尾地址即可
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }

    pub fn push_context(&self, cx: TrapContext) -> &'static mut TrapContext {
        // 从内核栈栈顶出发,申请TrapCntext大小的栈帧
        let cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            // 将申请后的地址设置为TrapContext的地址
            *cx_ptr = cx;
        }
        unsafe { cx_ptr.as_mut().unwrap() }
    }
}

impl UserStack {
    fn get_sp(&self) -> usize {
        // 同上
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

impl AppManager {
    pub fn print_app_info(&self) {
        println!("[kernel] num_app = {}", self.num_app);
    }

    /// 返回当前运行的appid的地址
    pub fn get_current_app_addr(&self) -> usize {
        if self.current_app >= self.num_app {
            panic!("All application completed!!")
        }
        APP_BASE_ADDRESS + self.current_app * APP_SIZE_LIMIT
    }

    /// 移动到下一个运行的appid
    pub fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }
}

/// 初始化批处理系统，APP_MANAGER也是在这时候初始化的
pub fn init() {
    print_app_info();
}

/// 打印当前执行的app信息
pub fn print_app_info() {
    APP_MANAGER.exclusive_access().print_app_info();
}

pub fn run_next_app() -> ! {
    let mut app_manager = APP_MANAGER.exclusive_access();

    let current_addr = app_manager.get_current_app_addr();
    println!("[kernel] next app addr {:#x}", current_addr);
    app_manager.move_to_next_app();
    drop(app_manager);

    extern "C" {
        fn __restore(cx_addr: usize);
    }

    // 在内核栈上压入一个Trap上下文
    // 其 sepc 是应用程序入口地址 0x80400000 ，其 sp 寄存器指向用户栈，其 sstatus 的 SPP 字段被设置为 User
    unsafe {
        __restore(KERNEL_STACK.push_context(TrapContext::app_init_context(
            current_addr,
            USER_STACK.get_sp(),
        )) as *const _ as usize);
    }
    panic!("Unreachable in batch::run_next_app!");
}

lazy_static! {
    static ref APP_MANAGER: UPSafeCell<AppManager> = unsafe {
        UPSafeCell::new({
            // 找到link_app.S中的符号_num_app
            extern "C" {
                fn _num_app();
            }
            // 根据起始符号解析出应用的数量以及各个起始地址
            let num_app_ptr = _num_app as usize as *const usize;
            let num_app = num_app_ptr.read_volatile();

            load_apps();
            AppManager { num_app, current_app: 0}
        })
    };
}
