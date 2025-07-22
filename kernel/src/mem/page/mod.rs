use crate::mem::page::allocator::PageAllocator;
use crate::mem::page::page_table::{EXECUTE_DISABLE, PAGE_LEAKED, USER_ACCESSIBLE, WRITABLE};
use core::ops::Deref;
use core::ptr::NonNull;
use crate::UEFIBootInfo;

mod page_table;
mod physical;
pub mod allocator;

/// TODO: Rework of this system is required
/// TODO: PageAllocator -> safe abstraction over PageTable for allocating VIRTUAL pages
/// TODO: PhysicalPageAllocator -> allocator for reserving physical pages, no guarantee that they're mapped
/// TODO: PageTable -> low-level representation of paging tables
/// TODO:
/// TODO: page tables should use PhysicalPageAllocator and a const TEMP_PAGE_TABLE for creating inner tables (pdpt, pd, pt)
/// TODO: page table functions should ALL be unsafe because they must unsafely dereference pointers
/// TODO: PageAllocator should make sure the unsafe page table functions are safe
/// TODO: PageAllocator should have a kernel() and current() method for getting the necessary PageAllocator
/// TODO: PageAllocator should be designed to be created over and over (because its going to be used per-process

pub fn init_paging(boot_info: &UEFIBootInfo) {
    physical::setup_ppa(boot_info);
    allocator::init_paging(boot_info);
}

pub type VirtAddr = u64;
pub type PhysAddr = u64;

#[derive(Debug)]
pub enum PageAllocationError {
    OutOfMemory,
    OutOfVirtualMemory,
    InvalidDeallocationPointer,
}

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
    allocator: &'a mut PageAllocator,
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
        let allocator = self.allocator as *mut PageAllocator;
        let allocator = unsafe { allocator.as_mut_unchecked() };

        allocator.dealloc(self);
    }
}