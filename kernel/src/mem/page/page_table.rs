use crate::mem::page::{VirtAddr};

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
pub struct PageTableEntry(u64);

#[repr(transparent)]
pub struct PageTable([PageTableEntry; 512]);

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
    
}