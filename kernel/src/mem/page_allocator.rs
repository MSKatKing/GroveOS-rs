use core::num::{NonZeroU64, NonZeroUsize};
use core::ops::{Deref, DerefMut, Index, IndexMut};
use core::ptr::NonNull;
use core::slice;
use crate::UEFIBootInfo;

#[derive(Copy, Clone)]
pub struct PageIdx(NonZeroU64);

impl<'a> PageIdx {
    fn new(val: u64) -> Option<PageIdx> {
        NonZeroU64::new(val).map(PageIdx)
    }
    
    pub fn next() -> Option<PageIdx> {
        FrameAllocator::get().get_next_idx()
    }
    
    #[inline(always)]
    fn get(&self) -> u64 {
        self.0.get()
    }
    
    pub fn is_free(self) -> bool {
        FrameAllocator::get().is_page_free(self)
    }
    
    pub fn allocate(self) -> Option<Page<'a>> {
        if !self.is_free() {
            None
        } else {
            let memory_address = NonNull::new((self.0.get() * Page::PAGE_SIZE as u64) as *mut u8).expect("should not be null");
            unsafe { Some(Page::from_raw_ptr(memory_address)) }
        }
    }
}

pub struct Page<'a>(&'a mut [u8]);

impl<'a> Page<'a> {
    pub const PAGE_SIZE: usize = 0x1000;
    
    unsafe fn from_raw_ptr(ptr: NonNull<u8>) -> Self {
        let slice = unsafe { slice::from_raw_parts_mut(ptr.as_ptr(), Self::PAGE_SIZE) };
        slice.fill(0);
        
        Page(slice)
    }
    
    pub fn data(&self) -> &[u8] {
        &self.0
    }
    
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.0.as_ptr()
    }
    
    pub fn idx(&self) -> PageIdx {
        PageIdx::new(self.as_ptr() as u64 / Self::PAGE_SIZE as u64).expect("should not be null")
    }
    
    pub unsafe fn as_type<T: Sized>(&self) -> &T {
        unsafe { self.as_ptr().cast::<T>().as_ref().expect("should not be null") }
    }
    
    pub unsafe fn as_type_mut<T: Sized>(&mut self) -> &'a mut T {
        unsafe { self.as_mut_ptr().cast::<T>().as_mut().expect("should not be null") }
    }
    
    pub unsafe fn leak(mut self) -> NonNull<u8> {
        let idx = self.idx();
        let ptr = NonNull::new(self.as_mut_ptr()).expect("should not be null");
        drop(self);
        
        FrameAllocator::get().set_page_used(idx, true);
        ptr
    }
    
    pub unsafe fn drop_type<T: Sized>(data: &mut T) {
        let data = NonNull::new((data as *mut T).cast::<u8>()).expect("should not be null");
        let _ = unsafe { Page::from_raw_ptr(data) };
    }
}

impl Deref for Page<'_> {
    type Target = [u8];
    
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Page<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for Page<'_> {
    fn drop(&mut self) {
        FrameAllocator::get().set_page_used(self.idx(), false)
    }
}

pub struct FrameAllocator {
    bitmap: &'static mut [u8],
    curr_ptr: NonZeroUsize,
}

static mut SELF: Option<FrameAllocator> = None;

impl FrameAllocator {
    pub fn init(boot_info: &UEFIBootInfo) {
        unsafe {
            SELF = Some(FrameAllocator::from(boot_info))
        }
    }
    
    pub fn get() -> &'static mut FrameAllocator {
        unsafe {
            #[allow(static_mut_refs)]
            SELF.as_mut().expect("should not be accessing frame allocator prior to its instantiation")
        }
    }
    
    pub fn get_next_idx(&mut self) -> Option<PageIdx> {
        for idx in self.curr_ptr.get()..(self.bitmap.len() * 8) {
            let idx = PageIdx::new(idx as u64).unwrap();
            
            if self.is_page_free(idx) {
                self.curr_ptr.checked_add(1)?;
                return Some(idx);
            }
        }
        
        None
    }
    
    pub fn is_page_free(&self, idx: PageIdx) -> bool {
        (self[idx] & (1 << (idx.get() % 8))) == 0
    }
    
    pub fn set_page_used(&mut self, idx: PageIdx, used: bool) {
        if used {
            self[idx] |= 1 << (idx.get() % 8);
        } else {
            self[idx] &= !(1 << (idx.get() % 8));
        }
    }
}

impl Index<PageIdx> for FrameAllocator {
    type Output = u8;
    
    fn index(&self, idx: PageIdx) -> &Self::Output {
        &self.bitmap[idx.get() as usize / 8]
    }
}

impl IndexMut<PageIdx> for FrameAllocator {
    fn index_mut(&mut self, idx: PageIdx) -> &mut Self::Output {
        &mut self.bitmap[idx.get() as usize / 8]
    }
}

impl From<&UEFIBootInfo> for FrameAllocator {
    fn from(value: &UEFIBootInfo) -> Self {
        let bitmap = unsafe {
            slice::from_raw_parts_mut(value.memory_bitmap, value.memory_bitmap_size)
        };
        
        bitmap.fill(0);
        
        Self {
            bitmap,
            curr_ptr: NonZeroUsize::new(1).expect("should not be 0"),
        }
    }
}

pub fn allocate_next_page<'a>() -> Option<Page<'a>> {
    PageIdx::next()?.allocate()
}