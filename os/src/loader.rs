

use alloc::vec::Vec;
use lazy_static::lazy_static;

lazy_static! {
    static ref APP_NAMES: Vec<&'static str> = {
        let num_app = get_num_app();
        extern "C" {
            fn _app_names();
        }
        let mut start = _app_names as usize as *const u8;
        let mut v = Vec::new();
        unsafe {
            for _ in 0..num_app {
                let mut end = start;
                while end.read_volatile() != b'\0' {
                    // 读取一个字符
                    end = end.add(1);
                }
                let slice = core::slice::from_raw_parts(start, end as usize - start as usize);
                let str = core::str::from_utf8(slice).unwrap();
                v.push(str);
                start = end.add(1);
            }
        }
        v
    };
}

/// 打印app列表
pub fn list_apps() {
    println!("/********* APPS *********");
    for app in APP_NAMES.iter() {
        println!("{}", app);
    }
    println!("/******************");
}

/// 根据app名称获取app数据
pub fn get_app_data_by_name(name: &str) -> Option<&'static [u8]> {
    if let Some((idx, _)) = APP_NAMES.iter().enumerate().find(|(_, n)| {
        let app_name = **n;
        app_name.eq(name)
    }) {
        return Some(get_app_data(idx));
    }
    None
}

/// 获取链接到内核内的应用的数目
pub fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    unsafe {
        // 从_num_app读值,读取的是.quad 5这个
        (_num_app as usize as *const usize).read_volatile()
    }
}

/// 根据传入的应用编号取出对应应用的 ELF 格式可执行文件数据。
///
/// 它们和之前一样仍是基于 build.rs 生成的 link_app.S 给出的符号来确定其位置，并实际放在内核的数据段中。
pub fn get_app_data(app_id: usize) -> &'static [u8] {
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    assert!(app_id < num_app);
    unsafe {
        core::slice::from_raw_parts(
            app_start[app_id] as *const u8,
            app_start[app_id + 1] - app_start[app_id],
        )
    }
}
