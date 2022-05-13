use core::arch::asm;

use crate::{
    config::{APP_BASE_ADDRESS, APP_SIZE_LIMIT},
    println,
};

/// 将所有的app都加载到内存中
/// 不同app的内存地址是不同的
pub fn load_apps() {
    /// 汇编程序会静态编译出应用程序的地址信息
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as *const usize;
    let num_app = get_num_app();
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    unsafe { asm!("fence.i") }

    for i in 0..num_app {
        let base_i = get_bast_i(i);

        // 将这个引用程序要占用的内存区域清空
        (base_i..base_i + APP_SIZE_LIMIT)
            .for_each(|addr| unsafe { (addr as *mut u8).write_volatile(0) });
        // 读取程序
        let src = unsafe {
            core::slice::from_raw_parts(app_start[i] as *const u8, app_start[i + 1] - app_start[i])
        };

        println!(
            "[kernel] Loading app_{} from {:#x} to {:#x}",
            i, app_start[i], base_i
        );
        // 从这个应用程序在操作系统的运行地址为起始获取一段内存
        let dst = unsafe { core::slice::from_raw_parts_mut(base_i as *mut u8, src.len()) };
        // 将应用程序从加载到内存的位置复制到运行位置
        dst.copy_from_slice(src);
    }
}

/// 获取对应id的应用程序在操作系统中被运行时的内存地址
pub fn get_bast_i(app_id: usize) -> usize {
    APP_BASE_ADDRESS + app_id * APP_SIZE_LIMIT
}

/// 获取所有应用程序
pub fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    unsafe {
        // 从_num_app读值,读取的是.quad 5这个
        (_num_app as usize as *const usize).read_volatile()
    }
}
