use core::cell::{RefCell, RefMut};

pub struct UPSafeCell<T> {
    inner: RefCell<T>,
}

unsafe impl<T> Sync for UPSafeCell<T> {}

/// 当我们要访问数据时（无论读还是写），需要首先调用 exclusive_access 获得数据的可变借用标记，
/// 通过它可以完成数据的读写，在操作完成之后我们需要销毁这个标记，此后才能开始对该数据的下一次访问。
///
/// 相比 RefCell 它不再允许多个读操作同时存在。
impl<T> UPSafeCell<T> {
    /// 通过添加unsafe来保证使用者在没有销毁访问标记的时候又进行访问，会直接使得程序退出
    pub unsafe fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(value),
        }
    }

    /// 只允许访问可变引用
    pub fn exclusive_access(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
}
