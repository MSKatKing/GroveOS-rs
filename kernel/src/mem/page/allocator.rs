use alloc::vec::Vec;
use crate::mem::page::{Page, VirtAddr};
use crate::mem::page::page_table::PageTable;

pub struct PageAllocator {
    pml4: &'static mut PageTable,
    virt_ptr: usize,
}

impl PageAllocator {
    pub fn kernel() -> &'static mut PageAllocator {
        todo!()
    }
    
    pub fn current() -> &'static mut PageAllocator {
        todo!()
    }
    
    pub fn new() -> Self {
        todo!()
    }
    
    pub fn alloc(&mut self) -> Option<Page> {
        todo!()
    }
    
    pub fn alloc_many(&mut self, count: usize) -> Option<Vec<Page>> {
        todo!()
    }
    
    pub fn alloc_at(&mut self, ptr: VirtAddr) -> Option<Page> {
        todo!()
    }
    
    pub fn alloc_many_at(&mut self, ptr: VirtAddr, count: usize) -> Option<Vec<Page>> {
        todo!()
    }
    
    pub fn dealloc(&mut self, page: &Page) {
        todo!()
    }
    
    pub unsafe fn dealloc_raw(&mut self, ptr: VirtAddr) {
        todo!()
    }
}