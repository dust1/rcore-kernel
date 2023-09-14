use sbi_rt::legacy::console_getchar;

use crate::{
    mm::translated_byte_buffer,
    print,
    task::{processor::{current_user_token, suspend_current_and_run_next}},
};

const FD_STDOUT: usize = 1;
const FD_STDIN: usize = 0;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let buffers = translated_byte_buffer(current_user_token(), buf, len);
            for buffer in buffers {
                print!("{}", core::str::from_utf8(buffer).unwrap());
            }
            len as isize
        }
        _ => {
            panic!("Unsupported fd in sys_write!")
        }
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDIN => {
            assert_eq!(len, 1, "only support 1 size in sys_read");
            let mut c: usize;
            loop {
                c = console_getchar();
                if c == 0 {
                    // c == 0表示没有输入，暂时切换到其他任务
                    suspend_current_and_run_next();
                    continue;
                } else {
                    break;
                }
            }
            let ch = c as u8;
            // 手动查找页表，将输入字符串写入到地址空间
            let mut buffers = translated_byte_buffer(current_user_token(), buf, len);
            unsafe {
                buffers[0].as_mut_ptr().write_volatile(ch);
            }
            1
        }
        _ => {
            panic!("Unsupported fd in sys_read!")
        }
    }
}
