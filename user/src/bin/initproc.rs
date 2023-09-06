#![no_std]
#![no_main]

use user_lib::{exec, fork, wait, yield_};

/// 用户初始程序

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    // 程序初始化
    if fork() == 0 {
        // 通过exec执行user_shell
        exec("user_shell\0");
    } else {
        // 调用fork的用户是initproc自身
        loop {
            let mut exit_code = 0;
            // 循环等待所有在user_shell下的子进程，并回收他们占据的资源
            let pid = wait(&mut exit_code);
            if pid == -1 {
                // 没有回收成功则释放CPU重新等待
                yield_();
                continue;
            }
            // 某个子进程回收成功
            println!(
                "[initproc] Release a zombie process, pid = {}, exit_code = {}",
                pid, exit_code
            );
        }
    }
    0
}

