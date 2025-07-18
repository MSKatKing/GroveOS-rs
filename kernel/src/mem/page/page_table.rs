use core::arch::asm;
use crate::mem::page::{PageAllocationError, PhysAddr, VirtAddr};
use crate::mem::page::physical::PhysicalPageAllocator;

pub(super) const PRESENT: u64 = 1 << 0;
pub(super) const WRITABLE: u64 = 1 << 1;
pub(super) const USER_ACCESSIBLE: u64 = 1 << 2;
pub(super) const WRITE_THROUGH: u64 = 1 << 3;
pub(super) const CACHE_DISABLE: u64 = 1 << 4;
pub(super) const PAGE_LEAKED: u64 = 1 << 62;
pub(super) const EXECUTE_DISABLE: u64 = 1 << 63;

const ADDR_SPAN: u64 = 0x000F_FFFF_FFFF_F000;

#[repr(transparent)]
#[derive(Copy, Clone, Default)]
pub struct PageTableEntry(pub(super) u64);

#[repr(transparent)]
pub struct PageTable(pub(super) [PageTableEntry; 512]);

impl PageTableEntry {
    /// Clears all flags and sets the address this entry is pointing to to addr
    pub fn map_to_addr(&mut self, addr: PhysAddr) {
        self.0 = 0 | (addr & ADDR_SPAN) & PRESENT;
    }

    /// Swaps the address this entry is pointing to while preserving flags
    pub fn swap_addr(&mut self, addr: PhysAddr) {
        self.0 &= !ADDR_SPAN;
        self.0 |= addr & ADDR_SPAN;
    }

    pub fn clear(&mut self) {
        self.0 = 0;
    }

    pub fn has_flag(&self, flag: u64) -> bool {
        (self.0 & flag) != 0
    }

    pub fn get_addr(&self) -> Option<PhysAddr> {
        if self.has_flag(PRESENT) {
            Some(self.0 & ADDR_SPAN)
        } else {
            None
        }
    }

    pub fn set_flag(&mut self, flag: u64, value: bool) {
        self.0 &= !flag;
        self.0 |= flag & if value { !0 } else { 0 };
    }
}

impl PageTable {
    const PAGE_TABLE_PML4_PAGE: VirtAddr = 0xFFFF_FDFF_FFFF_D000;
    const PAGE_TABLE_STATIC_PAGE: VirtAddr = 0xFFFF_FDFF_FFFF_E000;
    const PAGE_TABLE_WORK_PAGE: VirtAddr = 0xFFFF_FDFF_FFFF_F000;

    pub fn new() -> Result<*mut PageTable, PageAllocationError> {
        let page = PhysicalPageAllocator::get().alloc()?;
        Ok(page as *mut PageTable)
    }

    pub fn setup_pml4(&mut self) -> Result<(), PageAllocationError> {
        let allocator = PhysicalPageAllocator::get();

        let pdpt = allocator.alloc()?;
        let pd = allocator.alloc()?;
        let pt = allocator.alloc()?;

        self.0[510].map_to_addr(pdpt);

        {
            let pdpt = Self::map_temp(pdpt);
            pdpt.0[511].map_to_addr(pd);
        }

        {
            let pd = Self::map_temp(pd);
            pd.0[511].map_to_addr(pt);
        }

        {
            let pt_addr = pt;
            let pt = Self::map_temp(pt);

            pt.0[509].map_to_addr(PageTable::current().translate(self as *const Self as VirtAddr).expect("should exist"));
            pt.0[510].map_to_addr(pt_addr);
        }

        Ok(())
    }

    pub fn current() -> &'static mut PageTable {
        unsafe { (Self::PAGE_TABLE_PML4_PAGE as *mut PageTable).as_mut_unchecked() }
    }

    pub fn map_addr(&mut self, vaddr: VirtAddr, paddr: PhysAddr, flags: u64) {
        todo!()
    }

    pub fn unmap_addr(&mut self, vaddr: VirtAddr) {
        todo!()
    }

    pub fn is_mapped(vaddr: VirtAddr) -> bool {
        todo!()
    }

    fn map_temp(addr: PhysAddr) -> &'static mut PageTable {
        let work_entry = unsafe { (Self::PAGE_TABLE_STATIC_PAGE as *mut PageTable).as_mut_unchecked() };
        work_entry.0[511].swap_addr(addr);

        Self::invplg(Self::PAGE_TABLE_WORK_PAGE);

        unsafe { (Self::PAGE_TABLE_WORK_PAGE as *mut PageTable).as_mut_unchecked() }
    }

    fn invplg(vaddr: VirtAddr) {
        unsafe {
            asm!("invlpg [{}]", in(reg) vaddr, options(nostack, preserves_flags));
        }
    }
}