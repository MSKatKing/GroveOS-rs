use crate::mem::heap::{PAGE_SIZE, SEGMENT_SIZE};

pub const HEAP_PAGE_DESC_SIZE: usize = (PAGE_SIZE / SEGMENT_SIZE) / 4; // 4096 (page size) / 8 (segment size) / 4 (4 entries per byte)

#[repr(packed)]
pub struct HeapPageDescriptor {
    bitmap: [u8; HEAP_PAGE_DESC_SIZE],
}

#[repr(u8)]
pub enum HeapPageDescriptorTag {
    Free = 0b00,
    Used = 0b01,
    End = 0b10,
    Unused = 0b11,
}