//! Process management syscalls
use alloc::sync::Arc;

use crate::{
    loader::get_app_data_by_name,
    mm::{translated_refmut, translated_str},
    task::{
        exit_current_and_run_next,
        manager::add_task,
        processor::{current_task, current_user_token, suspend_current_and_run_next},
    },
    timer::get_time_us,
};

/// 退出当前的应用并切换到下个应用。
pub fn sys_exit(exit_code: i32) -> ! {
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    current_task().unwrap().pid.0 as isize
}

/// 功能：获取当前的时间，保存在 TimeVal 结构体 ts 中，_tz 在我们的实现中忽略
/// 返回值：返回是否执行成功，成功则返回 0
/// syscall ID：169
pub fn sys_get_time() -> isize {
    get_time_us() as isize
}

pub fn sys_fork() -> isize {
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();

    // fork的子进程中系统调用返回值设置为0
    trap_cx.x[10] = 0;

    add_task(new_task);

    // 字进程的pid为父进程的返回值
    // 通过返回值是否为0可以用来判断是否是子进程
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    let token = current_user_token();
    // 构造要执行的应用名称
    let path = translated_str(token, path);

    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// 如果当前的进程不存在一个进程 ID 为 pid（pid==-1 或 pid > 0）的子进程，则返回 -1；
/// 如果存在一个进程 ID 为 pid 的僵尸子进程，则正常回收并返回子进程的 pid，并更新系统调用的退出码参数为 exit_code 。
///
/// 2 的返回值，它的含义是子进程还没退出，通知用户库 user_lib （是实际发出系统调用的地方），
/// 这样用户库看到是 -2 后，就进一步调用 sys_yield 系统调用，让当前父进程进入等待状态。
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
    }

    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        let exit_code = child.inner_exclusive_access().exit_code;
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
}
