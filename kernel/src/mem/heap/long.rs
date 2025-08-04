use crate::mem::heap::metadata::HeapMetadataEntryType;
use core::ptr::NonNull;

pub struct HeapLongTable {}

pub struct HeapLongTableEntry {
    ptr: Option<NonNull<u8>>,
    pages: u32,
    ty: HeapMetadataEntryType,
}

#[repr(u8)]
pub enum HeapLongTableEntryType {
    Owned = 0,
    Shared = 1,
}
