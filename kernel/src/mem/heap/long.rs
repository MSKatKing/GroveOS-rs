use core::ops::{Deref, DerefMut};
use crate::mem::heap::descriptor::HeapPageDescriptor;
use core::ptr::NonNull;
use crate::mem::heap::PAGE_SIZE;

const LONG_TABLE_ENTRIES: usize = size_of::<HeapPageDescriptor>() / size_of::<HeapLongTableEntry>();

pub struct HeapLongTable {
    entries: [HeapLongTableEntry; LONG_TABLE_ENTRIES],
}

pub struct HeapLongTableEntry {
    ptr: Option<NonNull<u8>>,
    pages: u32,
    ty: HeapLongTableEntryType,
}

#[repr(u8)]
pub enum HeapLongTableEntryType {
    Owned = 0,
    Shared(NonNull<HeapPageDescriptor>),
}

impl Deref for HeapLongTable {
    type Target = [HeapLongTableEntry; LONG_TABLE_ENTRIES];

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl DerefMut for HeapLongTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entries
    }
}

impl HeapLongTable {
    pub fn has_free_entry(&self) -> bool {
        for entry in self.entries.iter() {
            if entry.ptr.is_none() {
                return true;
            }
        }
        
        false
    }
}

impl HeapLongTableEntry {
    pub fn contains_ptr(&self, ptr: *const u8) -> bool {
        if let Some(page) = self.ptr {
            page.addr().get() <= ptr as usize && page.addr().get() + PAGE_SIZE < ptr as usize
        } else {
            false
        }
    }
}