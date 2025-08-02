use crate::mem::page::physical::PhysicalPageAllocator;
use crate::mem::page::{PageAllocationError, PhysAddr, VirtAddr};
use crate::println;
use core::arch::asm;

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
        self.0 = 0 | (addr & ADDR_SPAN) | PRESENT;
    }

    /// Swaps the address this entry is pointing to while preserving flags
    pub fn swap_addr(&mut self, addr: PhysAddr) {
        self.0 &= !ADDR_SPAN;
        self.0 |= addr & ADDR_SPAN;
        self.set_flag(PRESENT, true);
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
    pub(super) const PAGE_TABLE_PML4_PAGE: VirtAddr = 0xFFFF_FDFF_FFFF_D000;
    const PAGE_TABLE_STATIC_PAGE: VirtAddr = 0xFFFF_FDFF_FFFF_E000;
    const PAGE_TABLE_WORK_PAGE: VirtAddr = 0xFFFF_FDFF_FFFF_F000;

    pub fn new() -> Result<*mut PageTable, PageAllocationError> {
        let page = PhysicalPageAllocator::get().alloc()?;
        Ok(page as *mut PageTable)
    }

    pub fn install(&self) {
        unsafe {
            let phys = PageTable::current()
                .translate(self as *const Self as u64)
                .expect("should be mapped");
            asm!("mov cr3, {}", in(reg) phys);
        }
    }

    pub fn setup_pml4(&mut self) -> Result<(), PageAllocationError> {
        let (pml4_i, pdpt_i, pd_t, _) = Self::indexes_of(Self::PAGE_TABLE_PML4_PAGE);

        let pdpt = Self::map_temp(self.get_or_create(pml4_i)?);
        let pd = Self::map_temp(pdpt.get_or_create(pdpt_i)?);
        let pt = pd.get_or_create(pd_t)?;

        self.map_addr(
            Self::PAGE_TABLE_PML4_PAGE,
            PageTable::current()
                .translate(self as *const Self as VirtAddr)
                .expect("should exist"),
            WRITABLE,
        )?;
        self.map_addr(Self::PAGE_TABLE_STATIC_PAGE, pt, WRITABLE)?;

        Ok(())
    }

    pub fn current() -> &'static mut PageTable {
        unsafe { (Self::PAGE_TABLE_PML4_PAGE as *mut PageTable).as_mut_unchecked() }
    }

    pub fn map_addr(
        &mut self,
        vaddr: VirtAddr,
        paddr: PhysAddr,
        flags: u64,
    ) -> Result<(), PageAllocationError> {
        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) = Self::indexes_of(vaddr);

        let pdpt = Self::map_temp(self.get_or_create(pml4_idx)?);
        let pd = Self::map_temp(pdpt.get_or_create(pdpt_idx)?);
        let pt = Self::map_temp(pd.get_or_create(pd_idx)?);

        pt.0[pt_idx].map_to_addr(paddr);
        pt.0[pt_idx].set_flag(flags, true);
        Self::invplg(vaddr);

        Ok(())
    }

    pub fn unmap_addr(&mut self, vaddr: VirtAddr) {
        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) = Self::indexes_of(vaddr);

        if let Some(pdpt) = self.0[pml4_idx].get_addr() {
            let pdpt = Self::map_temp(pdpt);
            if let Some(pd) = pdpt.0[pdpt_idx].get_addr() {
                let pd = Self::map_temp(pd);
                if let Some(pt) = pd.0[pd_idx].get_addr() {
                    let pt = Self::map_temp(pt);

                    pt.0[pt_idx].map_to_addr(0);
                }
            }
        }

        Self::invplg(vaddr);
    }

    pub fn is_mapped(&self, vaddr: VirtAddr) -> bool {
        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) = Self::indexes_of(vaddr);

        if let Some(pdpt) = self.0[pml4_idx].get_addr() {
            let pdpt = Self::map_temp(pdpt);
            if let Some(pd) = pdpt.0[pdpt_idx].get_addr() {
                let pd = Self::map_temp(pd);
                if let Some(pt) = pd.0[pd_idx].get_addr() {
                    let pt = Self::map_temp(pt);

                    pt.0[pt_idx].get_addr().is_some()
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn translate(&self, vaddr: VirtAddr) -> Option<PhysAddr> {
        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) = Self::indexes_of(vaddr);

        let pdpt = self.0[pml4_idx].get_addr()?;
        let pdpt = Self::map_temp(pdpt);

        let pd = pdpt.0[pdpt_idx].get_addr()?;
        let pd = Self::map_temp(pd);

        let pt = pd.0[pd_idx].get_addr()?;
        let pt = Self::map_temp(pt);

        let page = pt.0[pt_idx].get_addr()?;
        let offset = vaddr & 0xFFF;

        Some(page + offset)
    }

    pub fn set_flags(&mut self, vaddr: VirtAddr, flags: u64, value: bool) -> Option<()> {
        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) = Self::indexes_of(vaddr);

        let pdpt = self.0[pml4_idx].get_addr()?;
        let pdpt = Self::map_temp(pdpt);

        let pd = pdpt.0[pdpt_idx].get_addr()?;
        let pd = Self::map_temp(pd);

        let pt = pd.0[pd_idx].get_addr()?;
        let pt = Self::map_temp(pt);

        pt.0[pt_idx].set_flag(flags, value);

        Some(())
    }

    pub fn get_flags(&self, vaddr: VirtAddr) -> Option<PageTableEntry> {
        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) = Self::indexes_of(vaddr);

        let pdpt = self.0[pml4_idx].get_addr()?;
        let pdpt = Self::map_temp(pdpt);

        let pd = pdpt.0[pdpt_idx].get_addr()?;
        let pd = Self::map_temp(pd);

        let pt = pd.0[pd_idx].get_addr()?;
        let pt = Self::map_temp(pt);

        Some(pt.0[pt_idx])
    }

    pub fn drop(&mut self) {
        let ppa = PhysicalPageAllocator::get();

        let pml4_addr = self
            .translate(Self::PAGE_TABLE_PML4_PAGE)
            .expect("should exist");

        for i in 0..512 {
            if let Some(pdpt_addr) = self.0[i].get_addr() {
                let mut pdpt = Self::map_temp(pdpt_addr);
                for j in 0..512 {
                    if let Some(pd_addr) = pdpt.0[j].get_addr() {
                        let pd = Self::map_temp(pd_addr);
                        for k in 0..512 {
                            if let Some(pt) = pd.0[k].get_addr() {
                                ppa.dealloc(pt).expect("should exist")
                            }
                        }
                        ppa.dealloc(pd_addr).expect("should exist");
                        pdpt = Self::map_temp(pdpt_addr);
                    }
                }
                ppa.dealloc(pdpt_addr).expect("should exist");
            }
        }

        ppa.dealloc(pml4_addr).expect("should exist");
    }

    fn get_or_create(&mut self, idx: usize) -> Result<PhysAddr, PageAllocationError> {
        if self.0[idx].get_addr().is_none() {
            let phys = PhysicalPageAllocator::get().alloc()?;
            self.0[idx].map_to_addr(phys);
            self.0[idx].set_flag(WRITABLE, true);
        }

        Ok(self.0[idx].get_addr().expect("should exist"))
    }

    fn indexes_of(vaddr: VirtAddr) -> (usize, usize, usize, usize) {
        let vaddr = vaddr as usize;
        fn index(vaddr: usize, level: usize) -> usize {
            (vaddr >> (12 + 9 * level)) & 0x1FF
        }

        (
            index(vaddr, 3),
            index(vaddr, 2),
            index(vaddr, 1),
            index(vaddr, 0),
        )
    }

    fn map_temp(addr: PhysAddr) -> &'static mut PageTable {
        let work_entry =
            unsafe { (Self::PAGE_TABLE_STATIC_PAGE as *mut PageTable).as_mut_unchecked() };
        work_entry.0[511].swap_addr(addr);
        work_entry.0[511].set_flag(WRITABLE, true);

        Self::invplg(Self::PAGE_TABLE_WORK_PAGE);

        unsafe { (Self::PAGE_TABLE_WORK_PAGE as *mut PageTable).as_mut_unchecked() }
    }

    #[inline(always)]
    fn invplg(vaddr: VirtAddr) {
        unsafe {
            asm!("invlpg [{}]", in(reg) vaddr, options(nostack, preserves_flags));
        }
    }
}
