use riscv::register::time;

use crate::{config::CLOCK_FREQ, sbi::set_timer};

/// 将1秒钟分为TICKS_PER_SEC片时间片
///
/// e.g. 这里将其分为100片，即1s/100 = 10ms一次时间片
const TICKS_PER_SEC: usize = 100;

const MICRO_PER_SEC: usize = 1_000_000;

/// 以微秒为单位返回当前计数器的值
pub fn get_time_us() -> usize {
    time::read() / (CLOCK_FREQ / MICRO_PER_SEC)
}

/// 以毫秒为单位返回当前计数器的值
pub fn get_time_ms() -> usize {
    get_time_us() / 100
}

/// 取得当前mtime的值
///
/// mtime：用来统计自处理器上电以来经过了多少个内置时钟周期
pub fn get_time() -> usize {
    time::read()
}

pub fn set_next_trigger() {
    // 设置下一次出现中断的计数器增量值
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}
