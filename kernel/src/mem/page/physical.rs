use crate::mem::heap::PAGE_SIZE;
use crate::mem::page::{PageAllocationError, PhysAddr};
use crate::UEFIBootInfo;

static mut INSTANCE: PhysicalPageAllocator = PhysicalPageAllocator {
    bitmap: &mut [],
    phys_ptr: 0,
};

pub struct PhysicalPageAllocator {
    bitmap: &'static mut [u8],
    phys_ptr: usize,
}

impl PhysicalPageAllocator {
    fn idx_to_addr(idx: usize) -> PhysAddr {
        (idx * PAGE_SIZE) as _
    }

    fn addr_to_idx(addr: PhysAddr) -> usize {
        (addr / PAGE_SIZE as u64) as _
    }

    fn addr(&self) -> PhysAddr {
        Self::idx_to_addr(self.phys_ptr)
    }

    pub fn get() -> &'static mut PhysicalPageAllocator {
        #[allow(static_mut_refs)]
        unsafe {
            &mut INSTANCE
        }
    }

    pub fn alloc(&mut self) -> Result<PhysAddr, PageAllocationError> {
        if self.is_free(self.addr()) {
            let addr = Self::idx_to_addr(self.phys_ptr);
            self.set_used(addr, true);
            self.phys_ptr += 1;
            Ok(addr)
        } else {
            for idx in self.phys_ptr..(self.bitmap.len() * 8) {
                if self.is_free(Self::idx_to_addr(idx)) {
                    let addr = Self::idx_to_addr(idx);
                    self.phys_ptr = idx + 1;
                    return Ok(addr);
                }
            }

            Err(PageAllocationError::OutOfMemory)
        }
    }

    pub fn dealloc(&mut self, addr: PhysAddr) -> Result<(), PageAllocationError> {
        let idx = Self::addr_to_idx(addr);

        if idx > self.bitmap.len() * 8 {
            Err(PageAllocationError::InvalidDeallocationPointer)
        } else {
            self.set_used(addr, false);
            if idx < self.phys_ptr {
                self.phys_ptr = idx;
            }

            Ok(())
        }
    }

    pub fn is_free(&self, addr: PhysAddr) -> bool {
        let idx = Self::addr_to_idx(addr);
        let offset = idx % 8;
        let idx = idx / 8;

        self.bitmap[idx] & (1 << offset) == 0
    }

    fn set_used(&mut self, addr: PhysAddr, used: bool) {
        let idx = Self::addr_to_idx(addr);
        let offset = idx % 8;
        let idx = idx / 8;

        self.bitmap[idx] &= !(1 << offset);
        if used {
            self.bitmap[idx] |= 1 << offset;
        }
    }
}

pub fn setup_ppa(boot_info: &UEFIBootInfo) {
    unsafe {
        INSTANCE.bitmap = core::slice::from_raw_parts_mut(boot_info.memory_bitmap, boot_info.memory_bitmap_size);
        INSTANCE.phys_ptr = 0;
    }
}