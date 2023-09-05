use core::arch::asm;

use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};
use lazy_static::lazy_static;
use riscv::register::satp;

use crate::{
    config::{MEMORY_END, PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT, USER_STACK_SIZE},
    mm::{
        address::{PhysPageNum, StepByOne},
        frame_allocator::frame_alloc,
        page_table::PTEFlags,
    },
    println,
    sync::up::UPSafeCell,
};

use super::{
    address::{PhysAddr, VPNRange, VirtAddr, VirtPageNum},
    frame_allocator::FrameTracker,
    page_table::{PageTable, PageTableEntry},
};

/// 一段连续地址的虚拟内存
pub struct MapArea {
    // 一段虚拟页号的连续区间, 表示该逻辑段在地址区间中的位置和长
    vpn_range: VPNRange,
    // 保存了虚拟页号与物理页帧的映射关系
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    // 映射方式
    map_type: MapType,
    // 表示控制该逻辑段的访问方式，
    // 它是页表项标志位 PTEFlags 的一个子集，
    // 仅保留 U/R/W/X 四个标志位
    map_perm: MapPermission,
}

/// 地址空间
///
/// 一系列有关联的逻辑段,这些逻辑段组成了一个应用程序
/// 即：MemorySet是一个应用程序所持有的虚拟内存地址空间
///
/// PageTable 下挂着所有多级页表的节点所在的物理页帧，
/// 而每个 MapArea 下则挂着对应逻辑段中的数据所在的物理页帧，
/// 这两部分合在一起构成了一个地址空间所需的所有物理页帧。
pub struct MemorySet {
    // 该地址空间的多级页表
    page_table: PageTable,
    // 逻辑段向量
    areas: Vec<MapArea>,
}

/// 虚拟页面映射到物理页帧的方式
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MapType {
    // 直接映射
    // 恒等映射方式
    Identical,
    // 表示对于每个虚拟页面都有一个新分配的物理页帧与之对应，
    // 虚地址与物理地址的映射关系是相对随机的。
    Framed,
}

bitflags! {
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

// 创建内核地址空间
lazy_static! {
    pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> =
        Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) });
}

//
// 取出linker.ld中表示各个段位置的符号
// 下面这部分的内存映射都属于直接(Identical Mapped)映射
// ┌──────────────┐ 256GiB
// │              │
// ├──────────────┤ MEMORY_END
// │ 可用的物理帧  │ -rw-
// ├──────────────┤
// │ .bss         │ -rw-
// ├──────────────┤
// │ .data        │ -rw-
// ├──────────────┤
// │ .rodata      │ -r--
// ├──────────────┤
// │ .text        │ -r-x
// ├──────────────┤ BASE_ADDRESS
// │              │
// └──────────────┘ 0
extern "C" {
    fn stext();
    fn etext();
    fn srodata();
    fn erodata();
    fn sdata();
    fn edata();
    fn sbss_with_stack();
    fn ebss();
    fn ekernel();
    fn strampoline();
}

impl MemorySet {
    /// 新建一个空的地址空间
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }

    /// 在当前地址空间插入一个新的逻辑段map_area
    ///
    /// 如果它是以 Framed 方式映射到物理内存，
    /// 还可以可选地在那些被映射到的物理页帧上写入一些初始化数据 data
    fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        // 从页表管理器中申请一段物理内存用来给MapArea中的虚拟内存来构建映射关系
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            // 如果创建MapArea的时候有初始化数据，则将数据复制到物理地址
            map_area.copy_data(&mut self.page_table, data);
        }
        self.areas.push(map_area);
    }

    /// 在当前地址空间插入一个 Framed 方式映射到物理内存的逻辑段。
    ///
    /// 注意该方法的调用者要保证同一地址空间内的任意两个逻辑段不能存在交集
    /// TIPS: 从后面即将分别介绍的内核和应用的地址空间布局可以看出这一要求得到了保证；
    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) {
        self.push(
            MapArea::new(start_va, end_va, MapType::Framed, permission),
            None,
        );
    }

    /// 生成内核的地址空间
    ///
    /// new_kernel 将映射跳板和地址空间中最低256GiB中的内核逻辑段
    /// 在 new_kernel 中，我们从低地址到高地址依次创建5个逻辑段并通过push方法将它们插入到内核地址空间中
    pub fn new_kernel() -> Self {
        let mut memory_set = Self::new_bare();
        memory_set.map_trampoline();
        // 映射内核部分
        println!(
            "[kernel] .text [{:#x}, {:#x})",
            stext as usize, etext as usize
        );
        println!(
            "[kernel] .rodata [{:#x}, {:#x})",
            srodata as usize, erodata as usize
        );
        println!(
            "[kernel] .data [{:#x}, {:#x})",
            sdata as usize, edata as usize
        );
        println!(
            "[kernel] .bss [{:#x}, {:#x})",
            sbss_with_stack as usize, ebss as usize
        );

        println!("[kernel] mapping .text section");
        memory_set.push(
            MapArea::new(
                (stext as usize).into(),
                (etext as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::X,
            ),
            None,
        );

        println!("[kernel] mapping .rodata section");
        memory_set.push(
            MapArea::new(
                (srodata as usize).into(),
                (erodata as usize).into(),
                MapType::Identical,
                MapPermission::R,
            ),
            None,
        );

        println!("[kernel] mapping .data section");
        memory_set.push(
            MapArea::new(
                (sdata as usize).into(),
                (edata as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        println!("[kernel] mapping .bss section");
        memory_set.push(
            MapArea::new(
                (sbss_with_stack as usize).into(),
                (ebss as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        println!("[kernel] mapping physcial memory");
        memory_set.push(
            MapArea::new(
                (ekernel as usize).into(),
                MEMORY_END.into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        memory_set
    }

    /// 分析应用的 ELF 文件格式的内容，解析出各数据段并生成对应的地址空间
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        let mut memory_set = Self::new_bare();

        //将跳板插入到应用地址空间；
        memory_set.map_trampoline();

        // 分析elf文件
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        // elf合法性判断
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!!");

        // 记录目前涉及到的最大的虚拟页号
        let mut max_end_vpn = VirtPageNum(0);

        // 直接得到 program header 的数目，
        // 然后遍历所有的 program header 并将合适的区域加入到应用地址空间中。
        let ph_count = elf_header.pt2.ph_count();
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();

            // 确认 program header 的类型是 LOAD
            // 表明它有被内核加载的必要
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                // 通过 ph.virtual_addr() 和 ph.mem_size() 来计算这一区域在应用地址空间中的位置
                let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();

                // 通过 ph.flags() 来确认这一区域访问方式的限制并将其转换为 MapPermission 类型
                let mut map_perm = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_perm |= MapPermission::R;
                }
                if ph_flags.is_write() {
                    map_perm |= MapPermission::W;
                }
                if ph_flags.is_execute() {
                    map_perm |= MapPermission::X;
                }

                // 创建逻辑段 map_area
                let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);
                max_end_vpn = map_area.vpn_range.get_end();

                // push到应用地址空间, push的时候需要完成数据拷贝
                // 当前 program header 数据被存放的位置可以通过 ph.offset() 和 ph.file_size() 来找到。
                memory_set.push(
                    map_area,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                );
            }
        }

        // 开始处理用户栈
        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_stack_bottom: usize = max_end_va.into();

        // 空出一个栈帧,这个栈帧就是保护页面
        user_stack_bottom += PAGE_SIZE;

        // 添加用户栈空间
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;
        // 在应用地址空间中映射次高页面来存放用户栈（Trap上下文）
        memory_set.push(
            MapArea::new(
                user_stack_bottom.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );

        memory_set.push(
            MapArea::new(
                TRAP_CONTEXT.into(),
                TRAMPOLINE.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        // 不仅返回应用地址空间 memory_set ，也同时返回用户栈虚拟地址 user_stack_top
        // 以及从解析 ELF 得到的该应用入口点地址，它们将被我们用来创建应用的任务控制块。
        (
            memory_set,
            user_stack_top,
            elf.header.pt2.entry_point() as usize,
        )
    }

    /// 跳板机制
    pub fn map_trampoline(&mut self) {
        // 直接在多级页表中插入一个从地址空间的最高虚拟页面映射到跳板汇编代码所在的键值对
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(strampoline as usize).into(),
            PTEFlags::R | PTEFlags::X,
        );
    }

    /// 启用SV39分页模式
    ///
    /// 同时在切换地址空间的时候也会调用到
    pub fn activate(&self) {
        let satp = self.page_table.token();
        unsafe {
            // 将构造的token写入satp CSR中, 此时SV39分页模式启用
            satp::write(satp);
            // 切换地址空间后清空快表,防止读取到过期的键值对
            // TIPS: 快表用来加速MMU访问,系统先从快表中查询，如果没有命中则往三级缓存中查询
            asm!("sfence.vma");
        }
    }

    /// 获取应用地址空间
    pub fn token(&self) -> usize {
        self.page_table.token()
    }

    /// 尝试根据虚拟页号寻找页表项
    ///
    /// 如果找不到则返回None
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }
}

impl MapArea {
    /// 从一段连续的虚拟地址创建
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {
        let start_vpn: VirtPageNum = start_va.floor();
        let end_vpn: VirtPageNum = end_va.ceil();
        let vpn_range = VPNRange::new(start_vpn, end_vpn);
        let data_frames = BTreeMap::new();
        Self {
            vpn_range,
            data_frames,
            map_type,
            map_perm,
        }
    }

    /// 将当前逻辑段到物理内存的映射从传入的该逻辑段所属的地址空间的多级页表中加入或删除。
    ///
    /// 构建当前逻辑段所属的一段连续的虚拟地址到物理地址的映射关系
    ///
    /// 可以看到它们的实现是遍历逻辑段中的所有虚拟页面，
    /// 并以每个虚拟页面为单位依次在多级页表中进行键值对的插入或删除，
    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.map_one(page_table, vpn);
        }
    }

    #[allow(unused)]
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }

    /// 将切片 data 中的数据拷贝到当前逻辑段实际被内核放置在的各物理页帧上，
    /// 从而在地址空间中通过该逻辑段就能访问这些数据。
    ///
    /// 调用它的时候需要满足：切片 data 中的数据大小不超过当前逻辑段的总大小，
    /// 且切片中的数据会被对齐到逻辑段的开头，然后逐页拷贝到实际的物理页帧。
    pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let mut current_vpn = self.vpn_range.get_start();
        let len = data.len();
        loop {
            // 从data中切一块页帧大小的数据
            let src = &data[start..len.min(start + PAGE_SIZE)];

            // 根据虚拟页号获取该虚拟页号对应的实际物理内存块
            let dst = &mut page_table
                .translate(current_vpn) // 尝试根据虚拟页号获取PTE
                .unwrap()
                .ppn()
                .get_bytes_array()[..src.len()];

            // 将data中切出来的数据拷贝到物理内存中
            dst.copy_from_slice(src);

            start += PAGE_SIZE;
            if start >= len {
                break;
            }

            // 虚拟页号+1
            current_vpn.step();
        }
    }

    /// 在虚拟页号 vpn 已经确定的情况下，它需要知道要将一个怎么样的页表项插入多级页表。
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum;
        match self.map_type {
            // 当以恒等映射 Identical 方式映射的时候，物理页号就等于虚拟页号；
            MapType::Identical => {
                ppn = PhysPageNum(vpn.0);
            }
            // 当以 Framed 方式映射时，需要分配一个物理页帧让当前的虚拟页面可以映射过去，
            // 此时页表项中的物理页号自然就是 这个被分配的物理页帧的物理页号。
            // 此时还需要将这个物理页帧挂在逻辑段的 data_frames 字段下。
            MapType::Framed => {
                // 应用程序在这里才会申请内存
                let frame = frame_alloc().unwrap();
                ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);
            }
        }
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();
        page_table.map(vpn, ppn, pte_flags);
    }

    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        match self.map_type {
            MapType::Framed => {
                self.data_frames.remove(&vpn);
            }
            _ => {}
        }
        page_table.unmap(vpn);
    }
}

pub fn remap_test() {
    let kernel_space = KERNEL_SPACE.exclusive_access();
    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();
    assert!(!kernel_space
        .page_table
        .translate(mid_text.floor())
        .unwrap()
        .writable(),);
    assert!(!kernel_space
        .page_table
        .translate(mid_rodata.floor())
        .unwrap()
        .writable(),);
    assert!(!kernel_space
        .page_table
        .translate(mid_data.floor())
        .unwrap()
        .executable(),);
    println!("[kernel test] remap_test passed!");
}
