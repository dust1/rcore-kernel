use alloc::sync::Arc;

use crate::{block_dev::BlockDevice, get_block_cache, BLOCK_BITS};

/// 位图
///    
/// 用于检查后续的块是否已被使用
/// 每个bit表示一个块,0表示未使用、1表示已使用
pub struct Bitmap {
    // 位图的起始块
    start_block_id: usize,
    // 位图连续块数量
    blocks: usize,
}

/// 表示位图区域的磁盘数据结构
///
/// u64为64个bit, 64 * 64 = 4096, 为512字节,刚好一个块的大小
/// 可以表示4096个块
type BitmapBlock = [u64; 64];

impl Bitmap {
    pub fn new(start_block_id: usize, blocks: usize) -> Self {
        Self {
            start_block_id,
            blocks,
        }
    }

    /// 从位图中分配一个位
    ///
    /// 返回的是bit所在的位置,等同于索引节点的数据块编号
    pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        for block_offset in 0..self.blocks {
            let block_id = block_offset + self.start_block_id;
            return get_block_cache(block_id, Arc::clone(block_device))
                .lock()
                .modify(0, |bitmap_block: &mut BitmapBlock| {
                    if let Some((idx, bitmap)) = bitmap_block
                        .iter()
                        .enumerate()
                        .find(|(_, bits64)| **bits64 != u64::MAX)
                    {
                        // 获取bitmap的二进制表示中1的个数
                        // 即已经分配了多少块
                        let alloc_size = bitmap.trailing_ones() as usize;
                        bitmap_block[idx] |= 1u64 << alloc_size;

                        Some(block_offset * BLOCK_BITS + idx * 64 + alloc_size as usize)
                    } else {
                        None
                    }
                });
        }
        None
    }

    /// 回收一个位图
    /// 
    /// 
    /// 感觉不太对?
    /// 000000111111
    /// 我如果回收倒数第二个1则就变成
    /// 000000111101
    /// 此时重新开始分配会出现错误
    /// 除非回收的顺序是按照分配的顺序回收
    /// 000000011111
    /// 就可以
    pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, bit: usize) {
        let (block_offset, bitmap_idx, alloc_size) = decomposition(bit);
        get_block_cache(self.start_block_id + block_offset, Arc::clone(block_device))
            .lock()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                assert!(bitmap_block[bitmap_idx] & (1u64 << alloc_size) > 0);
                bitmap_block[bitmap_idx] -= 1u64 << alloc_size;
            });
    }
}

/// 返回(位图所在块, 位图所在BitmapBlock的下标, 位图所在BitmapBlock中u64的二进制表示中的下标)
fn decomposition(mut bit: usize) -> (usize, usize, usize) {
    let block_offset = bit / BLOCK_BITS;
    bit = bit % BLOCK_BITS;
    (block_offset, bit / 64, bit % 64)
}
