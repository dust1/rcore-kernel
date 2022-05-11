use core::arch::asm;

use lazy_static::*;

use crate::{println, sync::up::UPSafeCell, trap::context::TrapContext};

const MAX_APP_NUM: usize = 16;
const APP_BASE_ADDRESS: usize = 0x80400000;
const APP_SIZE_LIMIT: usize = 0x20000;

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
    // 各个app的起始地址
    app_start: [usize; MAX_APP_NUM + 1],
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
        for i in 0..self.num_app {
            println!(
                "[kernel] app_{} [{:#x}, {:#x})",
                i,
                self.app_start[i],
                self.app_start[i + 1]
            );
        }
    }

    /// 返回当前运行的appid
    pub fn get_current_app(&self) -> usize {
        self.current_app
    }

    /// 移动到下一个运行的appid
    pub fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }

    /// 根据appid加载应用程序
    unsafe fn load_app(&self, app_id: usize) {
        if app_id >= self.num_app {
            panic!("all application completed")
        }

        println!("[kernel] Loading app_{}", app_id);
        // 清除指令缓存(i-cache)
        asm!("fence.i");

        // 清理内存
        core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, APP_SIZE_LIMIT).fill(0);
        let app_src = core::slice::from_raw_parts(
            self.app_start[app_id] as *const u8,
            self.app_start[app_id + 1] - self.app_start[app_id],
        );

        // 将app_id所在的内存块复制到APP_BASE_ADDRESS开头的内存部分
        // 这部分内存块就是app的代码
        let app_dst = core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, app_src.len());
        app_dst.copy_from_slice(app_src);
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
    let current_app = app_manager.get_current_app();
    unsafe {
        app_manager.load_app(current_app);
    }
    app_manager.move_to_next_app();
    drop(app_manager);

    extern "C" {
        fn __restore(cx_addr: usize);
    }

    // 在内核栈上压入一个Trap上下文
    // 其 sepc 是应用程序入口地址 0x80400000 ，其 sp 寄存器指向用户栈，其 sstatus 的 SPP 字段被设置为 User
    unsafe {
        __restore(KERNEL_STACK.push_context(TrapContext::app_init_context(
            APP_BASE_ADDRESS,
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
            let mut app_start:[usize; MAX_APP_NUM + 1] = [0; MAX_APP_NUM + 1];
            let app_start_raw:&[usize] = core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1);
            app_start[..=num_app].copy_from_slice(app_start_raw);
            AppManager { num_app, current_app: 0, app_start }
        })
    };
}
