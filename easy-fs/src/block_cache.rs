use alloc::{collections::VecDeque, sync::Arc};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{block_dev::BlockDevice, BLOCK_SZ};

/// 内存中同时能够驻留的最大数据块数量
const BLOCK_CACHE_SIZE: usize = 16;

/// 块缓存
pub struct BlockCache {
    // 位于内存中的缓冲区
    cache: [u8; BLOCK_SZ],
    // 块id
    block_id: usize,
    // 底层块设备的引用，通过它实现对块的读写
    block_device: Arc<dyn BlockDevice>,
    // 记录这个块从磁盘载入内存后有没有被修改过
    modified: bool,
}

pub struct BlockCacheManager {
    queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,
}

impl BlockCache {
    /// 从磁盘中加载一个块
    pub fn new(block_id: usize, block_device: Arc<dyn BlockDevice>) -> Self {
        let mut cache = [0u8; BLOCK_SZ];
        block_device.read_block(block_id, &mut cache);
        Self {
            cache,
            block_id,
            block_device,
            modified: false,
        }
    }

    /// 将缓冲区的内容写入到磁盘
    pub fn sync(&mut self) {
        if self.modified {
            self.modified = false;
            self.block_device.write_block(self.block_id, &self.cache);
        }
    }

    /// get_ref的闭包封装
    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
        f(self.get_ref(offset))
    }

    /// get_mut的闭包封装
    pub fn modify<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.get_mut(offset))
    }

    /// 从指定偏移量中获取指定类型的对象引用
    pub fn get_ref<T>(&self, offset: usize) -> &T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size < BLOCK_SZ);
        let addr = self.addr_of_offset(offset);
        // 将addr转为T类型指针
        // 将指针转为对象
        // 取对象引用
        unsafe { &*(addr as *const T) }
    }

    /// 从指定偏移量中获取指定类型的对象可变引用
    pub fn get_mut<T>(&mut self, offset: usize) -> &mut T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size < BLOCK_SZ);
        let addr = self.addr_of_offset(offset);
        unsafe { &mut *(addr as *mut T) }
    }
}

impl BlockCache {
    /// 获取指定偏移量所在的数据地址指针
    fn addr_of_offset(&self, offset: usize) -> usize {
        if offset <= BLOCK_SZ {
            panic!("Block offset {} out of BLOCK_SZ: {}", offset, BLOCK_SZ)
        }
        &self.cache[offset] as *const _ as usize
    }
}

impl Drop for BlockCache {
    fn drop(&mut self) {
        self.sync();
    }
}

impl BlockCacheManager {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn get_block_cache(
        &mut self,
        block_id: usize,
        block_device: Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>> {
        if let Some((_, block)) = self.queue.iter().find(|(id, _)| block_id.eq(id)) {
            return Arc::clone(block);
        }

        if self.queue.len() >= BLOCK_CACHE_SIZE {
            // 由于外部可能还在使用块，因此需要查询到强引用为1的数据块，并将其移除
            // 强引用为1：没有其他部分使用到这个块
            if let Some((idx, _)) = self
                .queue
                .iter()
                .enumerate()
                .find(|(_, (_, block))| Arc::strong_count(block) == 1)
            {
                self.queue.drain(idx..=idx);
            } else {
                panic!("Run out of BlockCache!");
            }
        }
        let block = Arc::new(Mutex::new(BlockCache::new(
            block_id,
            Arc::clone(&block_device),
        )));
        self.queue.push_back((block_id, Arc::clone(&block)));
        block
    }
}

lazy_static! {
    pub static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> =
        Mutex::new(BlockCacheManager::new());
}

/// 给其他模块进行调用的获取块的接口
pub fn get_block_cache(
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
) -> Arc<Mutex<BlockCache>> {
    BLOCK_CACHE_MANAGER
        .lock()
        .get_block_cache(block_id, block_device)
}
