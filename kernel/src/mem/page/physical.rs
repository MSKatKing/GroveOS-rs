use crate::mem::heap::PAGE_SIZE;
use crate::mem::page::{PageAllocationError, PhysAddr};

static mut INSTANCE: PhysicalPageAllocator = PhysicalPageAllocator {
    bitmap: &mut [],
    phys_ptr: 0,
};

pub struct PhysicalPageAllocator {
    bitmap: &'static mut [u8],
    phys_ptr: usize,
}

impl PhysicalPageAllocator {
    fn idx_to_addr(&self) -> PhysAddr {
        (self.phys_ptr * PAGE_SIZE) as _
    }
    
    fn addr_to_idx(addr: PhysAddr) -> usize {
        (addr / PAGE_SIZE as u64) as _
    }
    
    pub fn get() -> &'static mut PhysicalPageAllocator {
        todo!()
    }
    
    pub fn alloc(&mut self) -> Result<PhysAddr, PageAllocationError> {
        todo!()
    }
    
    pub fn dealloc(&mut self, addr: PhysAddr) -> Result<(), PageAllocationError> {
        todo!()
    }
}