use core::alloc::{GlobalAlloc, Layout};
use core::num::NonZeroU64;
use core::ptr::NonNull;
use crate::mem::page_allocator::{allocate_next_page, allocate_next_pages, Page};
use crate::mem::paging::PageTable;

#[global_allocator]
pub static mut GLOBAL_HEAP: GroveAllocator = GroveAllocator;

pub struct GroveAllocator;

unsafe impl GlobalAlloc for GroveAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        HeapMetadataPage::kernel().alloc(layout.size())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        HeapMetadataPage::kernel().free(NonNull::new(ptr).unwrap())
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { core::slice::from_raw_parts_mut(self.alloc(layout), layout.size()) };
        ptr.fill(0);

        ptr.as_mut_ptr()
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq)]
enum SegmentType {
    SegmentFree = 0,
    SegmentUsed = 1,
    SegmentEnd = 2,
    SegmentLarge = 3,
}

impl From<u8> for SegmentType {
    fn from(value: u8) -> Self {
        match value {
            1 => SegmentType::SegmentUsed,
            2 => SegmentType::SegmentEnd,
            3 => SegmentType::SegmentLarge,
            _ => SegmentType::SegmentFree,
        }
    }
}

const BITMAP_SIZE: usize = (512 * 2) / 8;
const BITMAP_AMOUNT: usize = (4096 - (size_of::<u8>() * 4 + size_of::<u64>() * 2)) / size_of::<HeapBitmap>();

const SEGMENTS_PER_PAGE: usize = 4096 / 8;

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct HeapBitmap {
    flags: u8,
    page: Option<NonNull<u64>>,
    bitmap: [u8; BITMAP_SIZE],
}

#[repr(C, packed)]
struct HeapMetadataPage {
    verify: [u8; 4],
    next: Option<NonNull<HeapMetadataPage>>,
    previous: Option<NonNull<HeapMetadataPage>>,
    bitmaps: [HeapBitmap; BITMAP_AMOUNT],
}

const VERIFY_STR: [u8; 4] = ['_' as u8, 'G' as u8, 'H' as u8, 'P' as u8];
static mut KERNEL_HEAP: HeapMetadataPage = HeapMetadataPage {
    verify: VERIFY_STR,
    next: None,
    previous: None,
    bitmaps: [
        HeapBitmap {
            flags: 0,
            page: None,
            bitmap: [0u8; BITMAP_SIZE]
        }
    ; BITMAP_AMOUNT]
};

macro_rules! bytes_to_segments {
    ($byte:expr) => {
        if $byte % 8 == 0 {
            $byte / 8
        } else {
            $byte / 8 + 1
        }
    };
}

impl HeapMetadataPage {
    pub fn new() -> &'static mut Self {
        let mut page = allocate_next_page().unwrap();
        PageTable::current().get_mut(page.as_ptr() as u64)
            .map_to(page.as_ptr() as u64)
            .set_writable(true);

        page.fill(0);

        let out = unsafe { (page.leak().0.as_ptr() as *mut Self).as_mut().expect("should not be null") };
        out.verify = VERIFY_STR;

        out
    }

    pub fn kernel() -> &'static mut Self {
        unsafe {
            #[allow(static_mut_refs)]
            &mut KERNEL_HEAP
        }
    }

    pub fn drop(&mut self) {
        if let Some(mut previous) = self.previous {
            unsafe { previous.as_mut() }.previous = self.next;
        }

        if let Some(mut next) = self.next {
            unsafe { next.as_mut() }.previous = self.previous;
        }
    }

    pub fn alloc_new_page(&mut self) -> Option<usize> {
        for i in 0..BITMAP_AMOUNT {
            if let None = self.bitmaps[i].page {
                let page = allocate_next_page().unwrap();
                self.bitmaps[i].page = unsafe { Some(page.leak().0.cast::<u64>()) };
                self.bitmaps[i].flags |= 1;
                return Some(i);
            }
        }
        None
    }

    pub fn dealloc_page(&mut self, idx: usize) {
        if let Some(page) = self.bitmaps[idx].page {
            self.bitmaps[idx].page = None;
            drop(unsafe { Page::from_raw_ptr(page.cast::<u8>()) })
        }
    }

    pub fn find_free_segments(&mut self, count: usize, page: &mut usize) -> Option<usize> {
        for i in 0..BITMAP_AMOUNT {
            if let None = self.bitmaps[i].page {
                self.alloc_new_page();
            }

            'outer: for mut j in 0..SEGMENTS_PER_PAGE {
                if j + count - 1 >= SEGMENTS_PER_PAGE { break; }
                
                if self.get_bitmap_value(i, j) == SegmentType::SegmentFree {
                    for k in (j + 1)..count {
                        if self.get_bitmap_value(i, k) != SegmentType::SegmentFree {
                            j = k;
                            continue 'outer;
                        }
                    }
                    
                    *page = i;
                    return Some(j);
                }
            }
        }
        None
    }
    
    pub fn find_large_segments(&mut self, ptr: NonNull<u8>, page: &mut usize) -> Option<usize> {
        for i in 0..BITMAP_AMOUNT {
            if let None = self.bitmaps[i].page {
                self.alloc_new_page();
            }
            
            for j in 0..SEGMENTS_PER_PAGE {
                if self.get_bitmap_value(i, j) == SegmentType::SegmentLarge {
                    if unsafe { *(self.bitmaps[i].page.unwrap().as_ptr() as *const u64).offset(j as isize) } == ptr.as_ptr() as u64 {
                        *page = i;
                        return Some(j);
                    }
                }
            }
        }
        None
    }
    
    pub fn find_page_from_heap(&mut self, addr: NonNull<u8>) -> Option<usize> {
        for i in 0..BITMAP_AMOUNT {
            if let None = self.bitmaps[i].page { continue; }
            
            if self.bitmaps[i].page.unwrap().as_ptr() as u64 == addr.as_ptr() as u64 {
                return Some(i);
            }
        }
        None
    }
    
    pub fn get_bitmap_value(&mut self, idx: usize, addr: usize) -> SegmentType {
        SegmentType::from(self.bitmaps[idx].bitmap[addr / 4] >> ((addr % 4) * 2) & 0x3)
    }
    
    pub fn set_bitmap_value(&mut self, idx: usize, addr: usize, val: SegmentType) {
        self.bitmaps[idx].bitmap[addr / 4] &= !(0x3 << ((addr % 4) * 2));
        self.bitmaps[idx].bitmap[addr / 4] |= (val as u8) << ((addr % 4) * 2);
    }
    
    pub fn alloc(&mut self, size: usize) -> *mut u8 {
        let mut page = 0;
        let mut idx = 0;
        let mut curr_heap = self;
        
        loop {
            if let Some(data) = curr_heap.find_free_segments(if size > Page::PAGE_SIZE { 2 } else { bytes_to_segments!(size) }, &mut page) {
                idx = data;
                break;
            } else {
                if let Some(mut next) = curr_heap.next {
                    curr_heap = unsafe { next.as_mut() };
                } else {
                    let next = Self::new();
                    curr_heap.next = NonNull::new(next as *mut HeapMetadataPage);
                    next.previous = NonNull::new(curr_heap as *mut HeapMetadataPage);
                    curr_heap = next;
                }
            }
        }
        
        if size > Page::PAGE_SIZE {
            curr_heap.set_bitmap_value(page, idx, SegmentType::SegmentLarge);
            curr_heap.set_bitmap_value(page, idx + 1, SegmentType::SegmentEnd);
            
            let memory = allocate_next_pages((size / Page::PAGE_SIZE) + 1).unwrap();
            let (ptr, size) = unsafe { memory.leak() };
            let memory = ptr.as_ptr() as u64;
            
            unsafe { *curr_heap.bitmaps[page].page.unwrap().offset(idx as isize).as_mut() = memory };
            unsafe { *curr_heap.bitmaps[page].page.unwrap().offset(idx as isize + 1).as_mut() = size.get() };
            return memory as *mut u8;
        }
        
        for i in 0..=bytes_to_segments!(size) {
            curr_heap.set_bitmap_value(page, idx + i, if i == bytes_to_segments!(size) { SegmentType::SegmentEnd } else { SegmentType::SegmentUsed });
        }
        
        (curr_heap.bitmaps[page].page.unwrap().as_ptr() as usize + idx) as *mut u8
    }
    
    pub fn free(&mut self, ptr: NonNull<u8>) {
        let mut page = None;
        let mut idx = None;
        let mut curr_heap = NonNull::new(self as *mut HeapMetadataPage).expect("should not be null");
        
        loop {
            if let Some(new_page) = unsafe { curr_heap.as_mut() }.find_page_from_heap(ptr) {
                page = Some(new_page);
                break;
            } else {
                if let Some(mut next) = unsafe { curr_heap.as_mut() }.next {
                    curr_heap = NonNull::new(unsafe { next.as_mut() }).expect("should not be null");
                } else {
                    break;
                }
            }
        }
        
        if let None = page {
            curr_heap = NonNull::new(self as *mut HeapMetadataPage).expect("should not be null");
            let mut page = 0;
            loop {
                if let Some(new_idx) = unsafe { curr_heap.as_mut() }.find_large_segments(ptr, &mut page) {
                    idx = Some(new_idx);
                } else {
                    if let Some(mut next) = unsafe { curr_heap.as_mut() }.next {
                        curr_heap = NonNull::new(unsafe { next.as_mut() }).expect("should not be null");
                    } else {
                        break;
                    }
                }
            }
            
            if let None = idx {
                return;
            }
            
            let ptr = unsafe {
                let ptr = curr_heap.as_mut().bitmaps[page].page.unwrap().as_ptr();
                let ptr = ptr.offset(idx.unwrap() as isize) as *mut u8;
                
                let size = *curr_heap.as_mut().bitmaps[page].page.unwrap().as_ptr().offset(idx.unwrap() as isize + 1);
                
                drop(Page::with_page_count(NonNull::new_unchecked(ptr), NonZeroU64::new_unchecked(size)));
                
                NonNull::new_unchecked(ptr)
            };
            
            self.free(ptr);
            return;
        }
        
        let page = page.unwrap();
        let mut idx = 0;
        
        while unsafe { curr_heap.as_mut() }.get_bitmap_value(page, idx) != SegmentType::SegmentEnd {
            unsafe { curr_heap.as_mut() }.set_bitmap_value(page, idx, SegmentType::SegmentFree);
            idx += 1;
        }
        
        unsafe { curr_heap.as_mut() }.set_bitmap_value(page, idx, SegmentType::SegmentFree);
        unsafe { curr_heap.as_mut() }.dealloc_page(page);
        
        for i in unsafe { curr_heap.as_mut() }.bitmaps[page].bitmap {
            if i != 0 {
                return;
            }
        }
        
        if curr_heap == NonNull::new(self as *mut HeapMetadataPage).expect("should not be null") {
            return;
        }
        
        for bitmap in unsafe { curr_heap.as_mut() }.bitmaps.as_mut() {
            if let Some(_) = bitmap.page {
                return;
            }
        }
        
        unsafe { curr_heap.as_mut() }.drop();
    }
}