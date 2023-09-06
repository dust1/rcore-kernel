#![no_std]
#![no_main]

use alloc::string::String;
use user_lib::{
    console::getchar,
    exec, fork, waitpid,
};

extern crate alloc;

#[macro_use]
extern crate user_lib;

const LF: u8 = 0x0au8;
const CR: u8 = 0x0du8;
const DL: u8 = 0x7fu8;
const BS: u8 = 0x08u8;

#[no_mangle]
fn main() -> i32 {
    println!("Rust user shell!");
    // 当前用户输入的内容
    let mut line: String = String::new();
    print!(">> ");
    loop {
        let c = getchar();
        match c {
            LF | CR => {        // 回车键
                println!("");
                if !line.is_empty() {
                    line.push('\0');
                    // 试图fork一个子进程并执行
                    let pid = fork();
                    if pid == 0 {
                        if exec(line.as_str()) == -1 {
                            println!("Error when executing!");
                            return -4;
                        }
                        unreachable!()
                    } else {
                        // 等待fork子进程结束
                        let mut exit_code = 0;
                        let exit_pid = waitpid(pid as usize, &mut exit_code);
                        assert_eq!(pid, exit_pid);
                        println!("Shell: Process {} exited with code {}", pid, exit_code);
                    }
                    line.clear();
                }
                print!(">> ");
            }
            BS | DL => {        // 退格键（删除键）
                if !line.is_empty() {
                    // BS是退格符号
                    print!("{}", BS as char);
                    // 后退一个并将屏幕当前的最后一个字符用空格覆盖
                    print!(" ");
                    print!("{}", BS as char);
                    line.pop();
                }
            }
            _ => {
                print!("{}", c as char);
                line.push(c as char);
            }
        }
    }
}
