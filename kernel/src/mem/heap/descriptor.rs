use crate::mem::heap::{PAGE_SIZE, SEGMENT_SIZE};
use core::fmt::{Debug, Formatter};

pub const HEAP_PAGE_DESC_SIZE: usize = (PAGE_SIZE / SEGMENT_SIZE) / 4; // 4096 (page size) / 8 (segment size) / 4 (4 entries per byte)

#[repr(packed)]
pub struct HeapPageDescriptor {
    bitmap: [u8; HEAP_PAGE_DESC_SIZE],
}

#[repr(u8)]
#[derive(PartialEq, Debug)]
pub enum HeapPageDescriptorTag {
    Free = 0b00,
    Used = 0b01,
    End = 0b10,
    Unused = 0b11,
}

impl Default for HeapPageDescriptor {
    fn default() -> Self {
        Self {
            bitmap: [0; HEAP_PAGE_DESC_SIZE],
        }
    }
}

impl From<u8> for HeapPageDescriptorTag {
    fn from(v: u8) -> Self {
        match v {
            0b00 => HeapPageDescriptorTag::Free,
            0b01 => HeapPageDescriptorTag::Used,
            0b10 => HeapPageDescriptorTag::End,
            0b11 => HeapPageDescriptorTag::Unused,
            _ => unreachable!(),
        }
    }
}

impl HeapPageDescriptor {
    pub fn get_type(&self, offset: usize) -> HeapPageDescriptorTag {
        let bitmap_idx = offset / 4;
        let bit_offset = offset % 4;

        let value = (self.bitmap[bitmap_idx] & (0b11u8 << (2 * bit_offset))) >> (2 * bit_offset);
        HeapPageDescriptorTag::from(value)
    }

    pub fn set_type(&mut self, offset: usize, value: HeapPageDescriptorTag) {
        self.bitmap[offset / 4] &= !(0b11u8 << (2 * (offset % 4)));
        self.bitmap[offset / 4] |= (value as u8) << (2 * (offset % 4));
    }

    pub fn set_used(&mut self, offset: usize, len: usize) {
        for i in offset..offset + len {
            self.set_type(i, HeapPageDescriptorTag::Used);
        }
        self.set_type(offset + len - 1, HeapPageDescriptorTag::End);
    }

    pub fn set_free(&mut self, mut offset: usize) {
        while self.get_type(offset) != HeapPageDescriptorTag::End && offset < 511 {
            self.set_type(offset, HeapPageDescriptorTag::Free);
            offset += 1;
        }
        self.set_type(offset, HeapPageDescriptorTag::Free);
    }
    
    pub fn get_allocation_size(&mut self, mut offset: usize) -> usize {
        let mut len = 1;
        while self.get_type(offset) != HeapPageDescriptorTag::End {
            len += 1;
            offset += 1;
        }
        len
    }
    
    pub fn try_expand_allocation(&mut self, offset: usize, new_len: usize) -> bool {
        let old_len = self.get_allocation_size(offset);
        
        for i in offset + old_len..offset + new_len {
            if self.get_type(i) != HeapPageDescriptorTag::Unused {
                return false;
            }
        }
        
        self.set_used(offset, new_len);
        true
    }
    
    pub fn shrink_allocation(&mut self, offset: usize, new_len: usize) {
        self.set_used(offset + new_len - 1, 1);
        self.set_free(offset + new_len);
    }

    pub fn get_largest_free_segment(&self) -> (u16, u16) {
        let mut max_free_offset = 0;
        let mut max_free_len = 0;

        let mut curr_offset = -1;
        let mut curr_len = 0u16;

        for i in 0usize..512 {
            match self.get_type(i) {
                HeapPageDescriptorTag::Free => if curr_offset == -1 {
                    curr_offset = i as i16;
                    curr_len = 1;
                } else {
                    curr_len += 1;
                },
                _ => if curr_offset > 0 {
                    if curr_len > max_free_len {
                        max_free_offset = curr_offset as u16;
                        max_free_len = curr_len;
                    }

                    curr_offset = -1;
                }
            }
        }

        (max_free_offset, max_free_len)
    }
}

impl Debug for HeapPageDescriptor {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        for i in 0..512 {
            write!(f, "{:?}, ", self.get_type(i))?;
        }
        
        Ok(())
    }
}