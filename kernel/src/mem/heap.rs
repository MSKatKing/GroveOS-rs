use core::alloc::{GlobalAlloc, Layout};
use crate::println;

#[global_allocator]
pub static mut GLOBAL_HEAP: GroveAllocator = GroveAllocator {};

pub struct GroveAllocator {
    
}

unsafe impl GlobalAlloc for GroveAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        println!("allocating {:?}", layout);
        loop {}
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        println!("deallocating {:?}", layout);
        loop {}
    }
}