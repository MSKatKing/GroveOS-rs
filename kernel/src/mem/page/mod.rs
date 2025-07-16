use crate::mem::page::allocator::PageAllocator;
use crate::mem::page::page_table::{EXECUTE_DISABLE, PAGE_LEAKED, USER_ACCESSIBLE, WRITABLE};
use core::ops::Deref;
use core::ptr::NonNull;

mod page_table;
pub mod allocator;

pub type VirtAddr = u64;
pub type PhysAddr = u64;

#[repr(transparent)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct PagePtr(NonNull<u8>);

impl Deref for PagePtr {
    type Target = NonNull<u8>;
    
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Page<'a> {
    addr: VirtAddr,
    allocator: &'a PageAllocator,
}

impl Page<'_> {
    pub fn virt_addr(&self) -> VirtAddr {
        self.addr
    }
    
    pub fn leak(self) -> PagePtr {
        self.allocator.set_flag_for_page(&self, PAGE_LEAKED, true);
        PagePtr(unsafe { NonNull::new_unchecked(self.addr as *mut u8) })
    }
    
    pub fn set_writable(&mut self, writable: bool) {
        self.allocator.set_flag_for_page(&self, WRITABLE, writable);
    }
    
    pub fn set_executable(&mut self, executable: bool) {
        self.allocator.set_flag_for_page(&self, EXECUTE_DISABLE, executable);
    }
    
    pub fn set_user_accessible(&mut self, user_accessible: bool) {
        self.allocator.set_flag_for_page(&self, USER_ACCESSIBLE, user_accessible);
    }
}

impl Drop for Page<'_> {
    fn drop(&mut self) {
        todo!()
    }
}