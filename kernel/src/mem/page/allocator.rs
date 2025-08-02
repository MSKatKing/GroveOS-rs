use crate::mem::heap::PAGE_SIZE;
use crate::mem::page::page_table::{PageTable, PageTableEntry, PAGE_LEAKED, WRITABLE};
use crate::mem::page::physical::PhysicalPageAllocator;
use crate::mem::page::{Page, PageAllocationError, PhysAddr, VirtAddr};
use crate::UEFIBootInfo;
use alloc::vec::Vec;
use core::arch::asm;
use core::num::NonZeroU64;
use core::ptr::null_mut;

static mut KERNEL_PAGE_ALLOCATOR: PageAllocator = PageAllocator {
    pml4: &mut PageTable([PageTableEntry(0); 512]),
    virt_ptr: unsafe { NonZeroU64::new_unchecked(1) },
};

static mut CURRENT_PAGE_ALLOCATOR: *mut PageAllocator = null_mut();

pub struct PageAllocator {
    pub(super) pml4: &'static mut PageTable,
    virt_ptr: NonZeroU64,
}

impl PageAllocator {
    const MAX_VIRT_PAGE: u64 = 0xFFFF_FFFF_FFFF_F;

    pub fn kernel() -> &'static mut PageAllocator {
        #[allow(static_mut_refs)]
        unsafe {
            &mut KERNEL_PAGE_ALLOCATOR
        }
    }

    pub fn current() -> &'static mut PageAllocator {
        unsafe { CURRENT_PAGE_ALLOCATOR.as_mut_unchecked() }
    }

    pub fn install(&mut self) {
        self.pml4.install();
        self.pml4 =
            unsafe { (PageTable::PAGE_TABLE_PML4_PAGE as *mut PageTable).as_mut_unchecked() };

        unsafe {
            CURRENT_PAGE_ALLOCATOR = self as *mut Self;
        }
    }

    pub unsafe fn new_uninit() -> Self {
        let phys = PhysicalPageAllocator::get().alloc().expect("should exist");

        Self {
            pml4: unsafe { (phys as *mut PageTable).as_mut_unchecked() },
            virt_ptr: NonZeroU64::new(1).expect("not zero"),
        }
    }

    pub fn alloc(&mut self) -> Result<Page, PageAllocationError> {
        if !self.pml4.is_mapped(self.get_next_addr()) {
            let virt = self.get_next_addr();
            let phys = PhysicalPageAllocator::get().alloc()?;

            self.virt_ptr = self
                .virt_ptr
                .checked_add(1)
                .unwrap_or(NonZeroU64::new(1).expect("not zero"));

            self.pml4.map_addr(virt, phys, WRITABLE)?;
            Ok(Page {
                addr: virt,
                allocator: self,
            })
        } else {
            for idx in self.virt_ptr.get() + 1..Self::MAX_VIRT_PAGE {
                if !self.pml4.is_mapped(idx * PAGE_SIZE as u64) {
                    let virt = idx * PAGE_SIZE as u64;
                    let phys = PhysicalPageAllocator::get().alloc()?;
                    self.virt_ptr = NonZeroU64::new(idx + 1).expect("should not be zero");

                    self.pml4.map_addr(virt, phys, WRITABLE)?;
                    return Ok(Page {
                        addr: virt,
                        allocator: self,
                    });
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
        if self
            .pml4
            .get_flags(page.addr)
            .expect("should be mapped")
            .has_flag(PAGE_LEAKED)
        {
            return;
        }

        self.pml4.unmap_addr(page.addr);
        if page.addr < self.get_next_addr() {
            self.set_next_addr(page.addr);
        }
    }

    pub unsafe fn dealloc_raw(&mut self, ptr: VirtAddr) {
        self.pml4.unmap_addr(ptr);
        if ptr < self.get_next_addr() {
            self.set_next_addr(ptr);
        }
    }

    pub fn drop(self) {
        self.pml4.drop()
    }

    fn get_next_addr(&self) -> VirtAddr {
        self.virt_ptr.get() * PAGE_SIZE as u64
    }

    fn set_next_addr(&mut self, addr: VirtAddr) {
        self.virt_ptr = NonZeroU64::new(addr / PAGE_SIZE as u64)
            .unwrap_or(NonZeroU64::new(1).expect("not zero"));
    }

    pub(super) fn set_flag_for_page(&mut self, page: VirtAddr, flags: u64, value: bool) {
        self.pml4
            .set_flags(page, flags, value)
            .expect("page should be mapped");
    }
}

pub fn init_paging(boot_info: &UEFIBootInfo) {
    #[allow(static_mut_refs)]
    unsafe {
        KERNEL_PAGE_ALLOCATOR = PageAllocator::new_uninit();
        KERNEL_PAGE_ALLOCATOR
            .pml4
            .setup_pml4()
            .expect("failed to setup kernel page table");
    }

    let framebuffer_addr = boot_info.framebuffer.addr() as PhysAddr;
    for i in 0..(boot_info.framebuffer_size * size_of::<u32>()) / PAGE_SIZE {
        #[allow(static_mut_refs)]
        unsafe {
            KERNEL_PAGE_ALLOCATOR
                .pml4
                .map_addr(
                    framebuffer_addr + (i * PAGE_SIZE) as VirtAddr,
                    framebuffer_addr + (i * PAGE_SIZE) as PhysAddr,
                    WRITABLE,
                )
                .expect("failed to map framebuffer");
        }
    }

    unsafe {
        KERNEL_PAGE_ALLOCATOR.pml4.0[511] = PageTable::current().0[511];
    }

    let stack_ptr: VirtAddr;
    unsafe {
        asm!("mov {0}, rsp", out(reg) stack_ptr);
    }

    for i in 0..25 {
        #[allow(static_mut_refs)]
        unsafe {
            KERNEL_PAGE_ALLOCATOR
                .pml4
                .map_addr(
                    stack_ptr - (i * PAGE_SIZE) as VirtAddr,
                    stack_ptr - (i * PAGE_SIZE) as PhysAddr,
                    WRITABLE,
                )
                .expect("failed to map stack");
        }
    }

    let addr = boot_info.memory_bitmap.addr() as PhysAddr;
    for i in 0..((boot_info.memory_bitmap_size / PAGE_SIZE) + 1) {
        #[allow(static_mut_refs)]
        unsafe {
            KERNEL_PAGE_ALLOCATOR
                .pml4
                .map_addr(
                    addr + (i * PAGE_SIZE) as VirtAddr,
                    addr + (i * PAGE_SIZE) as PhysAddr,
                    WRITABLE,
                )
                .expect("failed to map memory bitmap");
        }
    }

    // TODO: map PageTable::current().0[511] into physical page allocator
}
