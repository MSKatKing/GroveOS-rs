use crate::mem::page_allocator::allocate_next_page;
use core::arch::asm;
use core::ops::{Index, IndexMut};

#[repr(transparent)]
#[derive(Debug, Default, Copy, Clone)]
pub struct PageTableEntry(u64);

macro_rules! paging_idx {
    ($addr:expr, $idx:expr) => {
        (($addr >> (39 - ($idx * 9))) & 0x1FF)
    };
}

impl PageTableEntry {
    const PRESENT: u64 = 1 << 0;
    const WRITABLE: u64 = 1 << 1;
    const USER_ACCESSIBLE: u64 = 1 << 2;
    
    pub fn is_present(&self) -> bool {
        (Self::PRESENT & self.0) != 0 && (self.get_addr() != 0)
    }
    
    pub fn get_addr(&self) -> u64 {
        self.0 & !0xFFF
    }
    
    pub fn map_to(&mut self, addr: u64) -> &mut Self {
        self.0 |= Self::PRESENT;
        self.0 |= addr & !0xFFF;
        self
    }
    
    pub fn set_writable(&mut self, writable: bool) -> &mut Self {
        if writable {
            self.0 |= PageTableEntry::WRITABLE;
        } else {
            self.0 &= !PageTableEntry::WRITABLE;
        }
        self
    }

    pub fn set_user_accessible(&mut self, user_accessible: bool) -> &mut Self {
        if user_accessible {
            self.0 |= PageTableEntry::USER_ACCESSIBLE;
        } else {
            self.0 &= !PageTableEntry::USER_ACCESSIBLE;
        }
        self
    }
}

pub struct PageTable([PageTableEntry; 512]);

impl PageTable {
    pub fn new() -> &'static mut Self {
        let page = allocate_next_page().unwrap();
        
        unsafe { (page.leak().0.as_ptr() as *mut PageTable).as_mut().expect("should not be null") }
    }
    
    #[inline(always)]
    pub fn current() -> &'static mut PageTable {
        let cr3: u64;
        unsafe {
            asm!("mov {cr3}, cr3", cr3 = out(reg) cr3);
        }
        
        unsafe { (cr3 as *mut PageTable).as_mut().unwrap() }
    }
    
    #[inline(always)]
    pub fn install(&mut self) {
        let cr3 = self as *mut _ as u64;
        unsafe {
            asm!("mov cr3, {a}", a = in(reg) cr3);
        }
    }
    
    pub fn get_or_insert_with<F: FnOnce() -> &'static mut PageTable>(&mut self, idx: usize, f: F) -> &mut PageTable {
        if !self.0[idx].is_present() {
            self.0[idx].map_to(f() as *mut _ as u64);
            self.0[idx].set_writable(true);
        }
        
        unsafe { (self.0[idx].get_addr() as *mut PageTable).as_mut().expect("should not be null") }
    }
    
    pub fn get_mut(&mut self, virt: u64) -> &mut PageTableEntry {
        let pml4_idx = paging_idx!(virt, 0) as usize;
        let pdpt_idx = paging_idx!(virt, 1) as usize;
        let pd_idx = paging_idx!(virt, 2) as usize;
        let pt_idx = paging_idx!(virt, 3) as usize;
        
        let pdpt = self.get_or_insert_with(pml4_idx, Self::new);
        let pd = pdpt.get_or_insert_with(pdpt_idx, Self::new);
        let pt = pd.get_or_insert_with(pd_idx, Self::new);
        
        &mut pt.0[pt_idx]
    }
}

impl Index<usize> for PageTable {   
    type Output = PageTableEntry;
    
    fn index(&self, idx: usize) -> &Self::Output {
        &self.0[idx]
    }
}

impl IndexMut<usize> for PageTable {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        &mut self.0[idx]
    }
}