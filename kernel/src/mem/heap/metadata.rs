use core::ops::{Index, IndexMut};
use core::ptr::NonNull;
use crate::mem::heap::descriptor::HeapPageDescriptor;
use crate::mem::heap::long::HeapLongTable;
use crate::mem::heap::PAGE_SIZE;

pub const METADATA_ENTRY_COUNT: usize = (PAGE_SIZE - 16) / const { size_of::<HeapMetadataEntry>() };

#[repr(align(4096))]
pub struct HeapMetadata {
    prev: Option<NonNull<HeapMetadata>>,
    next: Option<NonNull<HeapMetadata>>,
    entries: [HeapMetadataEntry; METADATA_ENTRY_COUNT],
}

#[repr(packed)]
#[derive(Default)]
pub struct HeapMetadataEntry {
    max_free_offset: u16,
    max_free_len: u16,
    page: HeapMetadataEntryType,
}

#[repr(u8)]
#[derive(Default)]
pub enum HeapMetadataEntryType {
    #[default]
    Unallocated = 0,
    General(NonNull<HeapPageDescriptor>),
    LongTable(NonNull<HeapLongTable>),
}

impl Index<usize> for HeapMetadata {
    type Output = HeapMetadataEntry;
    
    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl IndexMut<usize> for HeapMetadata {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

impl HeapMetadata {
    pub fn kernel() -> &'static mut Self {
        todo!()
    }
    
    pub fn allocate(&mut self, len: usize) -> Option<&[u8]> {
        todo!()
    }
    
    pub fn deallocate(&mut self, ptr: NonNull<u8>) {
        todo!()
    }
}