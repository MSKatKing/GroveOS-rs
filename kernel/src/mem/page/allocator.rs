use alloc::vec::Vec;
use crate::mem::page::{Page, PhysAddr, VirtAddr};
use crate::mem::page::page_table::{PageTable, PageTableEntry};
use crate::UEFIBootInfo;

static mut MEMORY_BITMAP: PhysicalMemoryBitmap = PhysicalMemoryBitmap {
    phys_ptr: 0,
    bitmap: &mut [],
};
static mut KERNEL_PAGE_ALLOCATOR: PageAllocator = PageAllocator {
    pml4: &mut PageTable([PageTableEntry(0); 512]),
    virt_ptr: 0,
};

struct PhysicalMemoryBitmap {
    bitmap: &'static mut [u8],
    phys_ptr: usize,
}

pub struct PageAllocator {
    pml4: &'static mut PageTable,
    virt_ptr: usize,
}

impl PageAllocator {
    pub fn kernel() -> &'static mut PageAllocator {
        #[allow(static_mut_refs)]
        unsafe { &mut KERNEL_PAGE_ALLOCATOR }
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

impl PhysicalMemoryBitmap {
    pub fn get() -> &'static mut Self {
        #[allow(static_mut_refs)]
        unsafe { &mut MEMORY_BITMAP }
    }

    pub fn get_next_available(&self) -> Option<PhysAddr> {
        todo!()
    }

    pub fn set_used(&mut self, addr: PhysAddr, used: bool) {
        todo!()
    }

    pub fn is_used(&self, addr: PhysAddr) -> bool {
        todo!()
    }
}

pub fn init_paging(boot_info: &UEFIBootInfo) {
    unsafe {
        MEMORY_BITMAP.bitmap = core::slice::from_raw_parts_mut(boot_info.memory_bitmap, boot_info.memory_bitmap_size);
        MEMORY_BITMAP.phys_ptr = 0;
    }

    // TODO: setup kernel page allocator
    // TODO: map boot info data into kernel page tables
    // TODO: fill physical memory bitmap with known used pages
}