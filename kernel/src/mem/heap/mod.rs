use core::alloc::{GlobalAlloc, Layout};

mod descriptor;
mod metadata;
mod long;

pub const PAGE_SIZE: usize = 0x1000;
pub const SEGMENT_SIZE: usize = 0x8;

#[global_allocator]
pub static KERNEL_HEAP: GroveHeap = GroveHeap;

pub struct GroveHeap;

unsafe impl GlobalAlloc for GroveHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unimplemented!()
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unimplemented!()
    }
}