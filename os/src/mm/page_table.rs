use alloc::vec::Vec;
use bitflags::*;

use super::{
    address::{PhysPageNum, StepByOne, VirtAddr, VirtPageNum},
    frame_allocator::{frame_alloc, FrameTracker},
};

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

/// 页表管理器
pub struct PageTable {
    // 根页表
    root_ppn: PhysPageNum,
    // 后续的所有页帧
    frames: Vec<FrameTracker>,
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

    /// 该页表项是否允许写入
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }

    /// 该页表项是否允许执行
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }

    /// 该页表项是否允许读取
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
}

impl PageTable {
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        let ppn = frame.ppn;
        let mut frames = Vec::new();
        frames.push(frame);
        PageTable {
            root_ppn: ppn,
            frames,
        }
    }

    /// 往多级页表中插入一个键值对
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {} is mapped before mapping", vpn.0);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }

    /// 移除多级页表中的一个键值对
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {} is valid before unmapping", vpn.0);
        *pte = PageTableEntry::empty();
    }

    /// 临时创建一个专用来手动查页表的 PageTable
    ///
    /// 它仅有一个从传入的 satp token 中得到的多级页表根节点的物理页号，
    /// 它的 frames 字段为空，也即不实际控制任何资源
    ///
    /// 之后，当遇到需要查一个特定页表（非当前正处在的地址空间的页表时）
    /// 便可先通过 PageTable::from_token 新建一个页表，再调用它的 translate 方法查页表。
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }

    /// 如果能够找到页表项，那么它会将页表项拷贝一份并返回，否则就返回一个 None 。
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }

    /// 在多级页表找到一个虚拟页号对应的页表项的可变引用。
    ///
    /// 如果在遍历的过程中发现有节点尚未创建则会新建一个节点。
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for i in 0..3 {
            let pte = &mut ppn.get_pte_array()[idxs[i]];
            if i == 2 {
                result = Some(pte);
                break;
            }

            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }

    /// 在多级页表找到一个虚拟页号对应的页表项的可变引用。
    ///
    /// 当找不到合法叶子节点的时候不会新建叶子节点而是直接返回 None 即查找失败。
    /// 因此，它不会尝试对页表本身进行修改，但是注意它返回的参数类型是页表项的可变引用，
    /// 也即它允许我们修改页表项。
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for i in 0..3 {
            let pte = &mut ppn.get_pte_array()[idxs[i]];
            if i == 2 {
                result = Some(pte);
                break;
            }

            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }

        result
    }

    /// 按照satp CSR格式要求构造一个无符号的64位整数，使得其分页模式位SV39
    ///
    /// 且将当前多级页表的根节点所在的物理页号填充进去
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
}

pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}
