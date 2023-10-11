use crate::EFS_MAGIX;


#[repr(C)]
pub struct SuperBlock {
    // 验证文件系统合法性
    magic: u32,
    // 文件系统的总块数,
    // 并不等于所占磁盘总块数,因为文件系统不一定占满整块磁盘
    pub total_blocks: u32,
    // easy-fs布局中后面四个连续区域有多少个块
    // inode位图块数
    pub inode_bitmap_blocks: u32,
    // inode区域块数
    pub inode_area_blocks: u32,
    // 数据位图块数
    pub data_bitmap_blocks: u32,
    // 数据区域块数
    pub data_area_blocks: u32,
}


impl SuperBlock {
    pub fn initialize(&mut self, total_blocks: u32, inode_bitmap_blocks: u32, inode_area_blocks: u32, data_bitmap_blocks: u32, data_area_blocks: u32) {
        *self = Self {
            magic: EFS_MAGIX,
            total_blocks,
            inode_bitmap_blocks,
            inode_area_blocks,
            data_bitmap_blocks,
            data_area_blocks
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic == EFS_MAGIX
    }
}