use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use crate::mem::heap::metadata::HeapMetadata;

pub mod descriptor;
pub mod metadata;
pub mod long;

pub const PAGE_SIZE: usize = 0x1000;
pub const SEGMENT_SIZE: usize = 0x8;

#[global_allocator]
pub static KERNEL_HEAP: GroveHeap = GroveHeap;

pub struct GroveHeap;

unsafe impl GlobalAlloc for GroveHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let allocation = unsafe { HeapMetadata::kernel() }.allocate(layout.size());

        if let Some(allocation) = allocation {
            allocation.as_mut_ptr()
        } else {
            panic!("Failed to allocate heap layout {:?}", layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) {
        unsafe { HeapMetadata::kernel() }.deallocate(NonNull::new(ptr).expect("Cannot deallocate null pointer!"));
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let allocation = unsafe { HeapMetadata::kernel() }.allocate(layout.size());

        if let Some(allocation) = allocation {
            allocation.fill(0);
            allocation.as_mut_ptr()
        } else {
            panic!("Failed to allocate heap layout {:?}", layout)
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let allocation = unsafe { HeapMetadata::kernel() }.reallocate(NonNull::new(ptr).expect("Cannot reallocate null ptr!"), new_size);

        if let Some(allocation) = allocation {
            allocation.as_mut_ptr()
        } else {
            panic!("Failed to allocate heap layout {:?}", layout)
        }
    }
}