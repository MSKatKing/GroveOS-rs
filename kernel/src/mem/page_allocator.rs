use crate::UEFIBootInfo;
use core::num::{NonZeroU64, NonZeroUsize};
use core::ops::{Deref, DerefMut, Index, IndexMut};
use core::ptr::NonNull;
use core::slice;

#[derive(Copy, Clone)]
pub struct PageIdx(NonZeroU64);

impl<'a> PageIdx {
    fn new(val: u64) -> Option<PageIdx> {
        NonZeroU64::new(val).map(PageIdx)
    }

    fn new_unchecked(val: u64) -> PageIdx {
        PageIdx(NonZeroU64::new(val).unwrap())
    }

    pub fn next() -> Option<PageIdx> {
        FrameAllocator::get().get_next_idx()
    }

    pub fn next_with_space(pages: u64) -> Option<PageIdx> {
        let mut idx = FrameAllocator::get().get_next_idx().unwrap();
        loop {
            if FrameAllocator::get().is_free_for(idx, pages) {
                return Some(idx);
            } else if idx.0.get() + pages + 1 > FrameAllocator::get().bitmap.len() as u64 * 8 {
                break;
            } else {
                idx = PageIdx::new(idx.0.get() + 1)?;
            }
        }
        None
    }

    #[inline(always)]
    fn get(&self) -> u64 {
        self.0.get()
    }

    pub fn is_free(self) -> bool {
        FrameAllocator::get().is_page_free(self)
    }

    /// Allocates this PageIdx if it is still free.
    ///
    /// Returns `None` if the PageIdx is no longer free.
    ///
    /// WARNING - The page returned by this function may or may not be mapped, it is the caller's responsibility to map this page into the PageTable.
    pub fn allocate(self) -> Option<Page<'a>> {
        if !self.is_free() {
            None
        } else {
            FrameAllocator::get().set_page_used(self, true);
            let memory_address = NonNull::new((self.0.get() * Page::PAGE_SIZE as u64) as *mut u8)
                .expect("should not be null");
            unsafe { Some(Page::from_raw_ptr(memory_address)) }
        }
    }

    /// Allocates this PageIdx with length `pages` if it is still free.
    ///
    /// Returns `None` if the PageIdx is no longer free.
    ///
    /// WARNING - The page returned by this function may or may not be mapped, it is the caller's responsibility to map the pages into the PageTable.
    pub fn allocate_multiple(self, pages: NonZeroU64) -> Option<Page<'a>> {
        if !FrameAllocator::get().is_free_for(self, pages.get()) {
            None
        } else {
            FrameAllocator::get().set_page_used(self, true);
            let start = NonNull::new((self.0.get() * Page::PAGE_SIZE as u64) as *mut u8)
                .expect("should not be null");
            let start = unsafe { Page::with_page_count(start, pages) };

            for i in 0..pages.get() - 1 {
                FrameAllocator::get().set_page_used(PageIdx::new_unchecked(i + 1), true);
            }

            Some(start)
        }
    }
}

pub struct Page<'a>(&'a mut [u8]);

impl<'a> Page<'a> {
    pub const PAGE_SIZE: usize = 0x1000;

    pub unsafe fn from_raw_ptr(ptr: NonNull<u8>) -> Self {
        let slice = unsafe { slice::from_raw_parts_mut(ptr.as_ptr(), Self::PAGE_SIZE) };
        slice.fill(0);

        Page(slice)
    }

    pub unsafe fn with_page_count(ptr: NonNull<u8>, pages: NonZeroU64) -> Self {
        let slice = unsafe {
            slice::from_raw_parts_mut(ptr.as_ptr(), Self::PAGE_SIZE * pages.get() as usize)
        };
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
        unsafe {
            self.as_ptr()
                .cast::<T>()
                .as_ref()
                .expect("should not be null")
        }
    }

    pub unsafe fn as_type_mut<T: Sized>(&mut self) -> &'a mut T {
        unsafe {
            self.as_mut_ptr()
                .cast::<T>()
                .as_mut()
                .expect("should not be null")
        }
    }

    pub unsafe fn leak(mut self) -> (NonNull<u8>, NonZeroU64) {
        let idx = self.idx();
        let ptr = NonNull::new(self.as_mut_ptr()).expect("should not be null");
        let len = self.0.len() / Self::PAGE_SIZE;
        drop(self);

        FrameAllocator::get().set_page_used(idx, true);
        (
            ptr,
            NonZeroU64::new(len as u64).expect("should not be zero"),
        )
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
        for i in 0..self.0.len() / Page::PAGE_SIZE {
            FrameAllocator::get()
                .set_page_used(PageIdx::new_unchecked(self.idx().0.get() + i as u64), false)
        }
    }
}

pub struct FrameAllocator {
    bitmap: &'static mut [u8],
    curr_ptr: NonZeroUsize,
}

static mut SELF: Option<FrameAllocator> = None;

impl FrameAllocator {
    pub fn init(boot_info: &UEFIBootInfo) {
        unsafe { SELF = Some(FrameAllocator::from(boot_info)) }
    }

    pub fn get() -> &'static mut FrameAllocator {
        unsafe {
            #[allow(static_mut_refs)]
            SELF.as_mut()
                .expect("should not be accessing frame allocator prior to its instantiation")
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

    pub fn is_free_for(&self, idx: PageIdx, pages: u64) -> bool {
        for i in 0..pages {
            if !self.is_page_free(PageIdx::new(idx.0.get() + i).unwrap()) {
                return false;
            }
        }
        true
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
        let bitmap =
            unsafe { slice::from_raw_parts_mut(value.memory_bitmap, value.memory_bitmap_size) };

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

pub fn allocate_next_pages<'a>(count: usize) -> Option<Page<'a>> {
    PageIdx::next_with_space(count as u64)?.allocate_multiple(NonZeroU64::new(count as u64)?)
}
