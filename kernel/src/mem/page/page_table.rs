use crate::mem::page::{PhysAddr, VirtAddr};
use crate::mem::page::allocator::PageAllocator;

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
    pub(super) const PML4_LEVEL: u8 = 3;
    const PT_LEVEL: u8 = 0;

    const PAGE_TABLE_STATIC_PAGE: VirtAddr = 0xFFFF_FFFF_7FFF_E000;
    pub(super) const PAGE_TABLE_WORK_PAGE: VirtAddr = 0xFFFF_FFFF_7FFF_F000;

    pub fn setup(&mut self) {
        todo!()
    }

    /// This function is unsafe because it assumes that PageTable::setup has been called.
    ///
    /// This function, if called before setup, will likely cause a page fault.
    unsafe fn get_work_page_entry() -> &'static mut PageTableEntry {
        unsafe { (Self::PAGE_TABLE_STATIC_PAGE as *mut PageTableEntry).offset(511).as_mut_unchecked() }
    }

    pub fn get_lowest_entry(&self, level: u8, addr: VirtAddr) -> Option<PageTableEntry> {
        debug_assert_eq!(self as *const Self as u64, Self::PAGE_TABLE_WORK_PAGE, "PageTable::get_lowest_entry only works if self is a reference to the work page");

        let index = Self::addr_to_idx(addr, level);

        if level == Self::PT_LEVEL {
            return Some(self.0[index]);
        }

        if let Some(next_table) = self.0[index].get_addr() {
            // SAFETY: should be safe because this function will be only available to the PageAllocator which ensures setup() is called
            let work_page_entry = unsafe { Self::get_work_page_entry() };

            let current_addr = work_page_entry.get_addr()?;
            work_page_entry.swap_addr(next_table);
            // self should point to Self::PAGE_TABLE_WORK_PAGE if setup was called, so self should be the next page table
            let out = self.get_lowest_entry(level - 1, addr);
            work_page_entry.swap_addr(current_addr);

            out
        } else {
            None
        }
    }

    pub fn get_lowest_entry_or_create(&mut self, allocator: &mut PageAllocator, level: u8, addr: VirtAddr) -> Option<PageTableEntry> {
        debug_assert_eq!(self as *const Self as u64, Self::PAGE_TABLE_WORK_PAGE, "PageTable::get_lowest_entry_or_create only works if self is a reference to the work page");

        let index = Self::addr_to_idx(addr, level);

        if level == Self::PT_LEVEL {
            return Some(self.0[index]);
        }

        if let None = self.0[index].get_addr() {
            self.create_page(allocator, index);
        }

        let next_table = self.0[index].get_addr().expect("should be created");
        let work_page_entry = unsafe { Self::get_work_page_entry() };

        let current_addr = work_page_entry.get_addr()?;
        work_page_entry.swap_addr(next_table);
        let out = self.get_lowest_entry_or_create(allocator, level - 1, addr);
        work_page_entry.swap_addr(current_addr);

        out
    }

    pub fn set_lowest_entry(&mut self, allocator: &mut PageAllocator, level: u8, addr: VirtAddr, entry: PageTableEntry) {
        debug_assert_eq!(self as *const Self as u64, Self::PAGE_TABLE_WORK_PAGE, "PageTable::set_lowest_entry only works if self is a reference to the work page");

        let index = Self::addr_to_idx(addr, level);

        if level == Self::PT_LEVEL {
            self.0[index] = entry;
        }

        if let None = self.0[index].get_addr() {
            self.create_page(allocator, index);
        }

        let next_table = self.0[index].get_addr().expect("should be created");
        let work_page_entry = unsafe { Self::get_work_page_entry() };

        let current_addr = work_page_entry.get_addr().expect("should be created");
        work_page_entry.swap_addr(next_table);
        let out = self.set_lowest_entry(allocator, level - 1, addr, entry);
        work_page_entry.swap_addr(current_addr);

        out
    }

    fn create_page(&mut self, allocator: &mut PageAllocator, index: usize) {
        let page = unsafe { allocator.alloc_no_map() }.expect("page should be allocated");
        self.0[index].map_to_addr(page.addr);
        self.0[index].set_flag(WRITABLE, true);

        page.leak();
    }

    fn addr_to_idx(addr: VirtAddr, level: u8) -> usize {
        assert!(level <= 3); // 5-level paging is not supported yet
        ((addr as usize) >> (21 + (level * 9))) & 0x1FF
    }
    
    pub unsafe fn swap_work_page(addr: PhysAddr) -> PhysAddr {
        let entry = unsafe { Self::get_work_page_entry() };
        let current = entry.get_addr().expect("should be created");
        entry.swap_addr(addr);
        current
    }

    pub fn is_mapped(&self, addr: VirtAddr) -> bool {
        if let Some(entry) = self.get_lowest_entry(Self::PML4_LEVEL, addr) {
            entry.has_flag(PRESENT)
        } else {
            false
        }
    }
}