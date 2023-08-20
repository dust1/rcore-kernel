use core::arch::asm;

use crate::{
    config::{APP_BASE_ADDRESS, APP_SIZE_LIMIT, KERNEL_STACK_SIZE, MAX_APP_NUM, USER_STACK_SIZE},
    println,
    trap::context::TrapContext,
};

/// 内核栈
#[repr(align(4096))]
#[derive(Clone, Copy)]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

/// 用户栈
#[repr(align(4096))]
#[derive(Clone, Copy)]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

/// 给每一个应用程序都分配一个内核栈/用户栈
static KERNEL_STACK: [KernelStack; MAX_APP_NUM] = [KernelStack {
    data: [0u8; KERNEL_STACK_SIZE],
}; MAX_APP_NUM];

static USER_STACK: [UserStack; MAX_APP_NUM] = [UserStack {
    data: [0u8; USER_STACK_SIZE],
}; MAX_APP_NUM];

impl KernelStack {
    // 获取栈的栈顶地址
    fn get_sp(&self) -> usize {
        // 由于在RISC-V中栈是向低地址增长的，我们只需要返回data数组的结尾地址即可
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }

    // 将trap上下文压入栈
    pub fn push_context(&self, cx: TrapContext) -> usize {
        // 从内核栈栈顶出发,申请TrapCntext大小的栈帧
        let cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            // 将申请后的地址设置为TrapContext的地址
            *cx_ptr = cx;
        }
        // 返回位于栈顶的Trap上下文
        cx_ptr as usize
    }
}

impl UserStack {
    fn get_sp(&self) -> usize {
        // 同上
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

/// 将所有的app都加载到内存中
/// 不同app的内存地址是不同的
pub fn load_apps() {
    // 汇编程序会静态编译出应用程序的地址信息
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as *const usize;
    let num_app = get_num_app();
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    // 保证 在它之后的取指过程必须能够看到在它之前的所有对于取指内存区域的修改
    unsafe { asm!("fence.i"); }

    for i in 0..num_app {
        // 获取每个APP的起始地址
        let base_i = get_base_i(i);

        // 将这个应用程序要占用的内存区域清空
        (base_i..base_i + APP_SIZE_LIMIT)
            .for_each(|addr| unsafe { (addr as *mut u8).write_volatile(0) });

        // 从data代码块中读取程序
        let src = unsafe {
            core::slice::from_raw_parts(app_start[i] as *const u8, app_start[i + 1] - app_start[i])
        };

        println!(
            "[kernel] Loading app_{} from {:#x} to {:#x}",
            i, app_start[i], base_i
        );

        // 获取应用程序将要运行的内存块
        let dst = unsafe { core::slice::from_raw_parts_mut(base_i as *mut u8, src.len()) };

        // 将应用程序从加载到内存的位置复制到运行位置
        dst.copy_from_slice(src);
    }
}

/// 获取对应id的应用程序在操作系统中被运行时的内存地址
/// 
/// 每个应用程序的大小都是固定的APP_SIZE_LIMIT
pub fn get_base_i(app_id: usize) -> usize {
    APP_BASE_ADDRESS + app_id * APP_SIZE_LIMIT
}

/// 获取应用程序的数量
pub fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    unsafe {
        // 从_num_app读值,读取的是.quad 5这个
        (_num_app as usize as *const usize).read_volatile()
    }
}

/// 构造该任务的 Trap 上下文（包括应用入口地址和用户栈指针）并将其压入到内核栈顶
/// 
/// sp：这个应用程序对应的用户栈
/// return: 返回的是这个应用程序对应的内核栈栈顶地址
pub fn init_app_cx(app_id: usize) -> usize {
    KERNEL_STACK[app_id].push_context(TrapContext::app_init_context(
        get_base_i(app_id),
        USER_STACK[app_id].get_sp(),
    ))
}
