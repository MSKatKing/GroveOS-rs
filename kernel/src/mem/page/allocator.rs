use alloc::vec::Vec;
use crate::mem::heap::PAGE_SIZE;
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
    virt_ptr: u64,
}

impl PageAllocator {
    pub fn kernel() -> &'static mut PageAllocator {
        #[allow(static_mut_refs)]
        unsafe { &mut KERNEL_PAGE_ALLOCATOR }
    }

    pub unsafe fn new_uninit() -> Self {
        let phys = PhysicalMemoryBitmap::get().get_next_available().expect("should exist");

        Self {
            pml4: unsafe { (phys as *mut PageTable).as_mut_unchecked() },
            virt_ptr: 0,
        }
    }
    
    pub fn alloc(&mut self) -> Option<Page> {
        // SAFETY: pml4 is edited but needs a reference to self. It does access pml4, but not simultaneously and not the same part.
        let allocator = unsafe { (self as *mut Self).as_mut_unchecked() };

        let old_work_addr = unsafe { PageTable::swap_work_page(PageTable::PAGE_TABLE_WORK_PAGE) };
        if let Some(mut entry) = self.pml4.get_lowest_entry_or_create(allocator, PageTable::PML4_LEVEL, self.get_next_addr()) {
            if let None = entry.get_addr() {
                let addr = PhysicalMemoryBitmap::get().get_next_available()?;
                entry.map_to_addr(addr);
                self.pml4.set_lowest_entry(allocator, PageTable::PML4_LEVEL, self.get_next_addr(), entry);
                unsafe { PageTable::swap_work_page(old_work_addr) };
                self.virt_ptr += 1;

                Some(Page { addr: self.get_next_addr(), allocator: self })
            } else {
                // virt_ptr was already taken, move on
                unsafe { PageTable::swap_work_page(old_work_addr) };
                None
            }
        } else {
            // Something went wrong creating the entry
            None
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

    pub(super) unsafe fn alloc_no_map(&mut self) -> Option<Page> {
        let phys = PhysicalMemoryBitmap::get().get_next_available()?;
        
        Some(Page {
            addr: phys,
            allocator: self,
        })
    }

    pub(super) fn set_flag_for_page(&self, page: &Page, flags: u64, value: bool) {
        todo!()
    }
}

impl PhysicalMemoryBitmap {
    pub fn get() -> &'static mut Self {
        #[allow(static_mut_refs)]
        unsafe { &mut MEMORY_BITMAP }
    }

    fn idx_to_addr(idx: usize) -> PhysAddr {
        (idx as u64) << 12
    }

    fn addr_to_idx(idx: PhysAddr) -> usize {
        (idx as usize) >> 12
    }

    pub fn get_next_available(&mut self) -> Option<PhysAddr> {
        if self.is_used(Self::idx_to_addr(self.phys_ptr)) {
            for i in self.phys_ptr + 1..self.bitmap.len() * 8 {
                if !self.is_used(Self::idx_to_addr(i)) {
                    self.phys_ptr = i + 1;
                    return Some(Self::idx_to_addr(i));
                }
            }

            None
        } else {
            self.phys_ptr += 1;
            Some(Self::idx_to_addr(self.phys_ptr))
        }
    }

    pub fn set_used(&mut self, addr: PhysAddr, used: bool) {
        let idx = addr as usize / 8;
        let offset = addr % 8;

        self.bitmap[idx] &= !(1 << offset);
        if used {
            self.bitmap[idx] |= 1 << offset;
        } else if self.phys_ptr > ((addr as usize) >> 12) {
            self.phys_ptr = (addr as usize) >> 12;
        }
    }

    pub fn is_used(&self, addr: PhysAddr) -> bool {
        let idx = addr as usize / 8;
        let offset = addr % 8;

        self.bitmap[idx] & (1 << offset) != 0
    }
}

pub fn init_paging(boot_info: &UEFIBootInfo) {
    #[allow(static_mut_refs)]
    unsafe {
        MEMORY_BITMAP.bitmap = core::slice::from_raw_parts_mut(boot_info.memory_bitmap, boot_info.memory_bitmap_size);
        MEMORY_BITMAP.phys_ptr = 0;

        KERNEL_PAGE_ALLOCATOR = PageAllocator::new_uninit();
        KERNEL_PAGE_ALLOCATOR.pml4.setup();
    }

    // TODO: setup kernel page allocator
    // TODO: map boot info data into kernel page tables
    // TODO: fill physical memory bitmap with known used pages
}