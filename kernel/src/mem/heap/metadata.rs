use core::ops::{Index, IndexMut};
use core::ptr::NonNull;
use crate::mem::heap::descriptor::HeapPageDescriptor;
use crate::mem::heap::long::HeapLongTable;
use crate::mem::heap::PAGE_SIZE;
use crate::mem::page_allocator::allocate_next_page;

pub const METADATA_ENTRY_COUNT: usize = (PAGE_SIZE - 16) / const { size_of::<HeapMetadataEntry>() };

#[repr(align(4096))]
pub struct HeapMetadata {
    prev: Option<NonNull<HeapMetadata>>,
    next: Option<NonNull<HeapMetadata>>,
    entries: [HeapMetadataEntry; METADATA_ENTRY_COUNT],
}

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
    
    pub fn allocate(&mut self, len: usize) -> Option<&mut [u8]> {
        todo!()
    }
    
    pub fn deallocate(&mut self, ptr: NonNull<u8>) {
        todo!()
    }
}

impl HeapMetadataEntry {
    pub fn can_store_alloc(&self, len: usize) -> bool {
        self.max_free_len <= len as u16
    }
    
    pub fn is_unallocated(&self) -> bool {
        match self.page { 
            HeapMetadataEntryType::Unallocated => true,
            _ => false,
        }
    }
    
    pub fn is_general_heap(&self) -> bool {
        match self.page { 
            HeapMetadataEntryType::General(_) => true,
            _ => false,
        }
    }
    
    pub fn contains_ptr(&self, ptr: *const u8) -> bool {
        let ptr = ptr as u64 & !0xFFF;
        let Some(page_ptr) = self.page.as_ptr() else {
            return false;
        };
        let page_ptr = page_ptr as u64 & !0xFFF;
        
        ptr == page_ptr
    }
    
    pub fn try_allocate_general_page(&mut self) -> Option<()> {
        let page = allocate_next_page()?;
        let (ptr, _) = unsafe { page.leak() };
        
        self.page = HeapMetadataEntryType::General(ptr.cast());
        self.max_free_offset = 0;
        self.max_free_len = 512;
        Some(())
    }
    
    pub fn allocate(&mut self, len: usize) -> Option<&'static mut [u8]> {
        match self.page { 
            HeapMetadataEntryType::General(ref mut p) => {
                let inner = unsafe { p.as_mut() };
                let offset = self.max_free_offset as usize;
                inner.set_used(offset, len);
                
                let (max_free_offset, max_free_len) = inner.get_largest_free_segment();
                self.max_free_offset = max_free_offset;
                self.max_free_len = max_free_len;
                
                Some(unsafe { core::slice::from_raw_parts_mut(p.as_ptr().cast::<u64>().offset(offset as isize).cast(), len) })
            },
            HeapMetadataEntryType::LongTable(ref mut p) => {
                todo!()
            }
            _ => None,
        }
    }
}

impl HeapMetadataEntryType {
    pub fn as_ptr(&self) -> Option<*const u8> {
        match self { 
            Self::General(p) => Some(p.as_ptr().cast()),
            Self::LongTable(p) => Some(p.as_ptr().cast()),
            _ => None,
        }
    }
}