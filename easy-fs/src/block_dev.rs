use core::any::Any;

/// 块设备接口
/// 
/// 用于对块进行读写,块缓存层会调用这两个方法，进行块缓存的管理
/// easy-fs本身并不会实现这两个方法.由具体的块设备驱动来实现
pub trait BlockDevice: Send + Sync + Any {
    fn read_block(&self, block_id: usize, buf: &mut [u8]);
    fn write_block(&self, block_id: usize, buf: &[u8]);
}
