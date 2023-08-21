use crate::config::{PAGE_SIZE, PAGE_SIZE_BITS};

/// 这四个地址的struct都是对usize的简单包装
///
/// 地址包含了页号和页内偏移

/// SV39模式下物理地址长度
const PA_WIDTH_SV39: usize = 56;

/// 物理页号长度
const PPN_WIDTH_SV39: usize = PA_WIDTH_SV39 - PAGE_SIZE_BITS;

/// 物理地址
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct PhysAddr(pub usize);

/// 虚拟地址
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct VirtAddr(pub usize);

/// 物理页号
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct PhysPageNum(pub usize);

/// 虚拟页号
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct VirtPageNum(pub usize);

impl From<usize> for PhysAddr {
    fn from(value: usize) -> Self {
        // 仅使用usize低位的56位来生成物理地址
        Self(value & (1 << PA_WIDTH_SV39 - 1))
    }
}

impl From<usize> for PhysPageNum {
    fn from(value: usize) -> Self {
        // 从usize低位的12位生成物理页号
        Self(value & (1 << PPN_WIDTH_SV39 - 1))
    }
}

impl From<PhysAddr> for usize {
    fn from(value: PhysAddr) -> Self {
        value.0
    }
}

impl From<PhysPageNum> for usize {
    fn from(value: PhysPageNum) -> Self {
        value.0
    }
}

impl From<PhysAddr> for PhysPageNum {
    fn from(value: PhysAddr) -> Self {
        assert_eq!(value.page_offset(), 0);
        value.floor()
    }
}

impl From<PhysPageNum> for PhysAddr {
    fn from(value: PhysPageNum) -> Self {
        Self(value.0 << PAGE_SIZE_BITS)
    }
}

impl PhysAddr {
    /// 获取物理地址的业内偏移
    ///
    /// 偏移取低地址的后12位
    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }

    /// 取物理地址向下取整后的页号
    ///
    /// 虽然内存分配中指定物理地址的高44位是页号，但是我们可以通过取模的方式直接获取页号，不需要通过字节截取
    pub fn floor(&self) -> PhysPageNum {
        PhysPageNum(self.0 / PAGE_SIZE)
    }

    /// 取物理地址向上取整之后的页号
    ///
    /// 同上
    pub fn ceil(&self) -> PhysPageNum {
        PhysPageNum((self.0 + PAGE_SIZE - 1) / PAGE_SIZE)
    }
}

/// 虚拟地址实现，同上
impl From<usize> for VirtAddr {
    fn from(value: usize) -> Self {
        Self(value & (1 << PA_WIDTH_SV39 - 1))
    }
}

impl From<usize> for VirtPageNum {
    fn from(value: usize) -> Self {
        Self(value & (1 << PPN_WIDTH_SV39 - 1))
    }
}

impl From<VirtAddr> for usize {
    fn from(value: VirtAddr) -> Self {
        value.0
    }
}

impl From<VirtPageNum> for usize {
    fn from(value: VirtPageNum) -> Self {
        value.0
    }
}

impl From<VirtAddr> for VirtPageNum {
    fn from(value: VirtAddr) -> Self {
        assert_eq!(value.page_offset(), 0);
        value.floor()
    }
}

impl From<VirtPageNum> for VirtAddr {
    fn from(value: VirtPageNum) -> Self {
        Self(value.0 << PAGE_SIZE_BITS)
    }
}

impl VirtAddr {
    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }

    pub fn floor(&self) -> VirtPageNum {
        VirtPageNum(self.0 / PAGE_SIZE)
    }

    pub fn ceil(&self) -> VirtPageNum {
        VirtPageNum((self.0 + PAGE_SIZE - 1) / PAGE_SIZE)
    }
}
