use crate::{println, UEFIBootInfo};

pub const PAGE_SIZE: usize = 0x1000;

pub struct FrameAllocator {
    bitmap: &'static mut [u8],
    curr_ptr: usize,
}

impl From<&UEFIBootInfo> for FrameAllocator {
    fn from(value: &UEFIBootInfo) -> Self {
        let bitmap = unsafe {
            core::slice::from_raw_parts_mut(value.memory_bitmap, value.memory_bitmap_size)
        };
        
        bitmap.fill(0);
        
        Self {
            bitmap,
            curr_ptr: 0
        }
    }
}

static mut FRAME_ALLOCATOR: Option<FrameAllocator> = None;

pub fn init_frame_allocator(frame_allocator: FrameAllocator) {
    unsafe { FRAME_ALLOCATOR = Some(frame_allocator) }
    set_page_used(0);
}

pub fn frame_allocator() -> &'static mut FrameAllocator {
    unsafe {
        #[allow(static_mut_refs)]
        FRAME_ALLOCATOR.as_mut().unwrap()
    }
}

pub fn is_page_free(page: usize) -> bool {
    (frame_allocator().bitmap[page / 8] & (1 << (page % 8))) == 0
}

pub fn set_page_used(page: usize) {
    frame_allocator().curr_ptr = page;
    frame_allocator().bitmap[page / 8] |= 1 << (page % 8);
}

pub fn set_page_free(page: usize) {
    if page < frame_allocator().curr_ptr {
        frame_allocator().curr_ptr = page;
    }
    
    frame_allocator().bitmap[page / 8] &= !(1 << (page % 8));
}

pub fn request_page() -> &'static mut [u8] {
    for page in frame_allocator().curr_ptr..(frame_allocator().bitmap.len() * 8) {
        if is_page_free(page) {
            set_page_used(page);
            unsafe {
                let page = (page * PAGE_SIZE) as *mut u8;
                return core::slice::from_raw_parts_mut(page, PAGE_SIZE);
            }
        }
    }
    
    panic!("no free page found in frame allocator");
}

pub fn free_page(page: &'static mut [u8]) {
    set_page_free(page.as_ptr() as usize / PAGE_SIZE);
}

pub fn request_pages(count: usize) -> &'static mut [u8] {
    'outer: for page in frame_allocator().curr_ptr..(frame_allocator().bitmap.len() * 8) {
        if is_page_free(page) {
            for j in 1..count {
                if !is_page_free(page + j) {
                    continue 'outer;
                }
            }
            
            let prev_ptr = frame_allocator().curr_ptr;
            
            for j in 0..count {
                set_page_used(page + j);
            }
            
            if prev_ptr + 1 < page {
                frame_allocator().curr_ptr = prev_ptr;
            }
            
            unsafe {
                let ptr = (page * PAGE_SIZE) as *mut u8;
                return core::slice::from_raw_parts_mut(ptr, PAGE_SIZE * count);
            }
        }
    }
    
    panic!("couldn't find {count} contiguous pages in frame allocator");
}

pub fn free_pages(page: &'static mut [u8], count: usize) {
    let base = page.as_ptr() as usize / PAGE_SIZE;
    for page in 0..count {
        set_page_free(page + base);
    }
}