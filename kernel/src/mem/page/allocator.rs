use crate::mem::heap::PAGE_SIZE;
use crate::mem::page::page_table::{PageTable, PageTableEntry};
use crate::mem::page::{Page, PageAllocationError, VirtAddr};
use crate::UEFIBootInfo;
use alloc::vec::Vec;
use crate::mem::page::physical::PhysicalPageAllocator;

static mut KERNEL_PAGE_ALLOCATOR: PageAllocator = PageAllocator {
    pml4: &mut PageTable([PageTableEntry(0); 512]),
    virt_ptr: 0,
};

pub struct PageAllocator {
    pml4: &'static mut PageTable,
    virt_ptr: u64,
}

impl PageAllocator {
    const MAX_VIRT_PAGE: u64 = 0xFFFF_FFFF_FFFF_F;

    pub fn kernel() -> &'static mut PageAllocator {
        #[allow(static_mut_refs)]
        unsafe { &mut KERNEL_PAGE_ALLOCATOR }
    }

    pub unsafe fn new_uninit() -> Self {
        let phys = PhysicalPageAllocator::get().alloc().expect("should exist");

        Self {
            pml4: unsafe { (phys as *mut PageTable).as_mut_unchecked() },
            virt_ptr: 0,
        }
    }
    
    pub fn alloc(&mut self) -> Result<Page, PageAllocationError> {
        if !self.pml4.is_mapped(self.get_next_addr()) {
            let virt = self.get_next_addr();
            let phys = PhysicalPageAllocator::get().alloc()?;
            self.virt_ptr += 1;

            self.pml4.map_addr(virt, phys, 0)?;
            Ok(Page { addr: virt, allocator: self })
        } else {
            for idx in self.virt_ptr..Self::MAX_VIRT_PAGE {
                self.virt_ptr = idx;

                if !self.pml4.is_mapped(self.get_next_addr()) {
                    let virt = self.get_next_addr();
                    let phys = PhysicalPageAllocator::get().alloc()?;
                    self.virt_ptr += 1;

                    self.pml4.map_addr(virt, phys, 0)?;
                    return Ok(Page { addr: virt, allocator: self });
                }
            }

            Err(PageAllocationError::OutOfVirtualMemory)
        }
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

    fn get_next_addr(&self) -> VirtAddr {
        self.virt_ptr * PAGE_SIZE as u64
    }

    fn set_next_addr(&mut self, addr: VirtAddr) {
        self.virt_ptr = addr / PAGE_SIZE as u64;
    }

    pub(super) fn set_flag_for_page(&self, page: &Page, flags: u64, value: bool) {
        todo!()
    }
}

pub fn init_paging(boot_info: &UEFIBootInfo) {
    #[allow(static_mut_refs)]
    unsafe {
        KERNEL_PAGE_ALLOCATOR = PageAllocator::new_uninit();
        KERNEL_PAGE_ALLOCATOR.pml4.setup_pml4().expect("TODO: panic message");
    }

    // TODO: setup kernel page allocator
    // TODO: map boot info data into kernel page tables
    // TODO: fill physical memory bitmap with known used pages
}