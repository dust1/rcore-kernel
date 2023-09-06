use crate::syscall;

const SYSCALL_EXEC: usize = 221;

pub fn sys_exec(path: &str) -> isize {
    syscall(SYSCALL_EXEC, [path.as_ptr() as usize, 0, 0])
}
