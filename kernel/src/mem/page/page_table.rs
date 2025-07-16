use crate::mem::page::{VirtAddr};
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
    pub fn map_to_addr(&mut self, addr: VirtAddr) {
        self.0 = 0 | (addr & ADDR_SPAN) & PRESENT;
    }
    
    pub fn clear(&mut self) {
        self.0 = 0;
    }
    
    pub fn has_flag(&self, flag: u64) -> bool {
        (self.0 & flag) != 0
    }
    
    pub fn get_addr(&self) -> Option<VirtAddr> {
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
    const PML4_LEVEL: u8 = 3;
    const PT_LEVEL: u8 = 0;

    const PAGE_TABLE_STATIC_PAGE: VirtAddr = 0xFFFF_FFFF_7FFF_E000;
    const PAGE_TABLE_WORK_PAGE: VirtAddr = 0xFFFF_FFFF_7FFF_F000;

    pub fn setup(&mut self) {
        todo!()
    }

    /// This function is unsafe because it assumes that PageTable::setup has been called.
    ///
    /// This function, if called before setup, will likely cause a page fault.
    unsafe fn get_work_page_entry() -> &'static mut PageTableEntry {
        unsafe { (Self::PAGE_TABLE_STATIC_PAGE as *mut PageTableEntry).offset(511).as_mut_unchecked() }
    }

    pub fn get_lowest_entry(&self, level: u8, addr: VirtAddr) -> Option<&PageTableEntry> {
        todo!()
    }

    pub fn get_lowest_entry_or_create(&mut self, allocator: &mut PageAllocator, level: u8, addr: VirtAddr) -> &mut PageTableEntry {
        todo!()
    }

    fn addr_to_idx(addr: VirtAddr, level: u8) -> usize {
        assert!(level <= 3); // 5-level paging is not supported yet
        ((addr as usize) >> (21 + (level * 9))) & 0x1FF
    }

    pub fn is_mapped(&self, addr: VirtAddr) -> bool {
        if let Some(entry) = self.get_lowest_entry(Self::PML4_LEVEL, addr) {
            entry.has_flag(PRESENT)
        } else {
            false
        }
    }
}