use bitflags::*;

use super::address::PhysPageNum;

bitflags! {
    /// 页表项的标志位
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

/// 页表项
///
/// 虚拟地址经过MMU查询之后得到的结果,其中包含了物理地址以及页表项标志位
/// 不包含页内偏移量
#[derive(Clone, Copy)]
#[repr(C)]
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    /// 根据页号和页表项标志位创建页表项
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        Self {
            // 偏移10是因为还有2个bit为RSW标志位
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }

    /// 生成一个全0的页表项
    ///
    /// 该页表项的V标志位为0，因此它是非法的
    pub fn empty() -> Self {
        Self { bits: 0 }
    }

    pub fn ppn(&self) -> PhysPageNum {
        // 页表项最高位包含一个10bit的Reserved标志位,需要将这部分去除
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }

    pub fn flags(&self) -> PTEFlags {
        // as u8就直接取低位的8位数据了
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }

    /// 判断该页表项是否合法
    pub fn is_valid(&self) -> bool {
        // 判断两个集合是否有交集
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }

}
