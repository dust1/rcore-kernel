/// qemu平台的时钟频率，单位为赫兹，也就是一秒内计数器的增量
///
/// 可以看成将1秒分成CLOCK_FREQ份
/// e.g. 后面的CLOCK_FREQ/100等于1s/100=100ms
pub const CLOCK_FREQ: usize = 12500000;

pub const MMIO: &[(usize, usize)] = &[
    (0x0010_0000, 0x00_2000), // VIRT_TEST/RTC  in virt machine
];
