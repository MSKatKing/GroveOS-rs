use core::cell::LazyCell;
use core::ops::{Index, IndexMut};
use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, Ordering};
use crate::mem::heap::descriptor::HeapPageDescriptor;
use crate::mem::heap::long::HeapLongTable;
use crate::mem::heap::PAGE_SIZE;
use crate::mem::page_allocator::allocate_next_page;
use crate::{print, println};

pub const METADATA_ENTRY_COUNT: usize = (PAGE_SIZE - 16) / const { size_of::<HeapMetadataEntry>() };

#[repr(align(4096))]
pub struct HeapMetadata {
    prev: Option<NonNull<HeapMetadata>>,
    next: Option<NonNull<HeapMetadata>>,
    entries: [HeapMetadataEntry; METADATA_ENTRY_COUNT],
}

#[derive(Default)]
pub struct HeapMetadataEntry {
    page: Option<NonNull<u64>>,
    max_free_offset: u16,
    max_free_len: u16,
    desc: HeapMetadataEntryType,
}

#[repr(u8)]
#[derive(Default)]
pub enum HeapMetadataEntryType {
    #[default]
    Unallocated = 0,
    General(HeapPageDescriptor),
    LongTable(HeapLongTable),
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

static mut KERNEL_HEAP_START: NonNull<HeapMetadata> = NonNull::dangling();

impl HeapMetadata {
    pub unsafe fn kernel() -> &'static mut Self {
        #[allow(static_mut_refs)]
        unsafe {
            KERNEL_HEAP_START.as_mut()
        }
    }
    
    pub unsafe fn init_heap() {
        unsafe {
            KERNEL_HEAP_START = Self::allocate_new_header().expect("failed to allocate start heap header");
        }
    }
    
    pub fn allocate_new_header() -> Option<NonNull<HeapMetadata>> {
        let page = allocate_next_page()?;
        let (ptr, _) = unsafe { page.leak() };
        
        println!("{}", ptr.as_ptr() as u64);
        
        const EMPTY_METADATA_ENTRY: HeapMetadataEntry = HeapMetadataEntry {
            page: None,
            max_free_offset: 0,
            max_free_len: 0,
            desc: HeapMetadataEntryType::Unallocated,
        };
        
        const EMPTY_METADATA: HeapMetadata = HeapMetadata {
            prev: None,
            next: None,
            entries: [EMPTY_METADATA_ENTRY; METADATA_ENTRY_COUNT],
        };
        
        let ptr = ptr.cast::<HeapMetadata>();
        unsafe { ptr.write(EMPTY_METADATA) }
        
        Some(ptr)
    }
    
    pub fn allocate(&mut self, len: usize) -> Option<&'static mut [u8]> {
        if len <= PAGE_SIZE {
            let len = len / 8 + 1;
            
            // Look through existing entries and check to see if any can allocate this len
            for (i, entry) in self.entries.iter_mut().enumerate() {
                if entry.is_general_heap() && entry.can_store_alloc(len) {
                    return entry.allocate(len);
                }
            }
            
            // Now try to allocate a new entry and allocate
            for entry in &mut self.entries {
                if entry.is_unallocated() {
                    return if let Some(()) = entry.try_allocate_general_page() {
                        entry.allocate(len)
                    } else {
                        // Return None here since try_allocate_general_page returns None if it couldn't get a page from the frame allocator,
                        // most likely meaning that the device is out of memory, so no allocations can be made
                        None
                    }
                }
            }
            
            // If we're here then that means that the current metadata header doesn't have space for this allocation
            if let Some(next) = &mut self.next {
                unsafe { next.as_mut() }.allocate(len)
            } else {
                todo!("try allocate new header here")
            }
        } else {
            // This is where long table allocation needs to happen
            todo!()
        }
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
        match self.desc { 
            HeapMetadataEntryType::Unallocated => true,
            _ => false,
        }
    }
    
    pub fn is_general_heap(&self) -> bool {
        match self.desc { 
            HeapMetadataEntryType::General(_) => true,
            _ => false,
        }
    }
    
    pub fn contains_ptr(&self, ptr: *const u8) -> bool {
        let ptr = ptr as u64 & !0xFFF;
        let Some(page_ptr) = self.page else {
            return false;
        };
        let page_ptr = page_ptr.as_ptr();
        let page_ptr = page_ptr as u64 & !0xFFF;
        
        ptr == page_ptr
    }
    
    pub fn try_allocate_general_page(&mut self) -> Option<()> {
        let page = allocate_next_page()?;
        let (ptr, _) = unsafe { page.leak() };
        
        self.page = Some(ptr.cast());
        self.desc = HeapMetadataEntryType::General(HeapPageDescriptor::default());
        self.max_free_offset = 0;
        self.max_free_len = 512;
        Some(())
    }
    
    pub fn allocate(&mut self, len: usize) -> Option<&'static mut [u8]> {
        match self.desc { 
            HeapMetadataEntryType::General(ref mut inner) => {
                let offset = self.max_free_offset as usize;
                inner.set_used(offset, len);
                
                let (max_free_offset, max_free_len) = inner.get_largest_free_segment();
                self.max_free_offset = max_free_offset;
                self.max_free_len = max_free_len;
                
                Some(unsafe { core::slice::from_raw_parts_mut(self.page?.as_ptr().cast::<u64>().offset(offset as isize).cast(), len) })
            },
            HeapMetadataEntryType::LongTable(ref mut p) => {
                todo!()
            }
            _ => None,
        }
    }
}