use core::ptr;
use crate::mem::page::page_table::PageTable;
use crate::mem::page::VirtAddr;
use crate::{print, println};
use crate::mem::heap::PAGE_SIZE;

#[repr(C, packed)]
pub struct Rsdp {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    _reserved: [u8; 3],
}

#[repr(C, packed)]
pub struct AcpiSdtHeader {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

unsafe fn check_checksum(ptr: *const u8, size: usize) -> bool {
    let bytes = unsafe {
        core::slice::from_raw_parts(ptr, size)
    };

    let mut sum = 0usize;
    for b in bytes {
        sum += *b as usize;
    }

    sum % 0x100 == 0
}

impl Rsdp {
    pub fn is_checksum_valid(&self) -> bool {
        unsafe { check_checksum(self as *const _ as *const u8, size_of::<Rsdp>()) }
    }
}

impl AcpiSdtHeader {
    pub fn is_checksum_valid(&self) -> bool {
        unsafe { check_checksum(self as *const _ as *const u8, self.length as usize) }
    }
}

pub unsafe fn parse_rsdp(rsdp_ptr: *const Rsdp) {
    unsafe {
        let rsdp = &*rsdp_ptr;

        if !rsdp.is_checksum_valid() {
            println!("RSDP is not valid: checksum failed");
            return;
        }

        let sig = core::str::from_utf8(&rsdp.signature).unwrap_or("Invalid");
        if sig != "RSD PTR " {
            return;
        }

        println!("RSDP Signature: {}", sig);
        println!("OEM ID: {}", core::str::from_utf8(&rsdp.oem_id).unwrap_or("Invalid OEM ID"));
        println!("Revision: {}", rsdp.revision);

        if rsdp.revision >= 2 && rsdp.length as usize >= size_of::<Rsdp>() {
            let a = rsdp.xsdt_address;
            println!("XSDT Address: {:#x}", a);

            PageTable::current().map_addr(a, a, 0).expect("failed to map xsdt");

            parse_xsdt(rsdp.xsdt_address as *const AcpiSdtHeader);
        }
    }
}

pub unsafe fn parse_rsdt(rsdt_ptr: *const AcpiSdtHeader) {
    unsafe {
        let header = &*rsdt_ptr;
        let entries = (header.length as usize - size_of::<AcpiSdtHeader>()) / 4;
        println!("RSDT with {} entries", entries);

        let entry_ptr = (rsdt_ptr as *const u8).add(size_of::<AcpiSdtHeader>());

        for i in 0..entries {
            let table_addr = *entry_ptr.add(i) as *const AcpiSdtHeader;
            print_acpi_table(table_addr);
        }
    }
}

pub unsafe fn parse_xsdt(xsdt_ptr: *const AcpiSdtHeader) {
    unsafe {
        let header = &*xsdt_ptr;

        if core::str::from_utf8(&header.signature).unwrap_or("ERR") != "XSDT" {
            println!("Failed to parse XSDT! Incorrect table name: {}", core::str::from_utf8(&header.signature).unwrap_or("ERR"));
            return;
        }

        if !header.is_checksum_valid() {
            println!("Failed to parse XSDT! Incorrect checksum!");
            return;
        }

        let entry_count = (header.length as usize - size_of::<AcpiSdtHeader>()) / 8;
        let entry_ptr = xsdt_ptr.cast::<u8>().offset(size_of::<AcpiSdtHeader>() as isize).cast::<u64>();

        println!("XSDT with {} entries @ {:#x}", entry_count, entry_ptr as usize);

        PageTable::current().map_addr(entry_ptr as VirtAddr, entry_ptr as VirtAddr, 0).expect("failed to map xsdt entry");

        for i in 0..entry_count {
            let entry = ptr::read_unaligned(entry_ptr.offset(i as isize)) as *const AcpiSdtHeader;
            print!("Entry {}", i);

            if entry.is_null() {
                println!(": Empty");
                continue;
            }

            print!(" @ {:#x}: ", entry as usize);

            PageTable::current().map_addr(entry as VirtAddr, entry as VirtAddr, 0).expect("failed to map xsdt entry");
            PageTable::current().map_addr(entry as VirtAddr + size_of::<AcpiSdtHeader>() as VirtAddr, entry as VirtAddr + size_of::<AcpiSdtHeader>() as VirtAddr, 0).expect("failed to map xsdt entry");

            print_acpi_table(entry);
        }
    }
}

pub unsafe fn print_acpi_table(table_ptr: *const AcpiSdtHeader) {
    unsafe {
        let header = &*table_ptr;

        let start_page = table_ptr as usize & !(PAGE_SIZE - 1);
        let end_page = (table_ptr as usize + header.length as usize + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        for i in start_page..=end_page {
            PageTable::current().map_addr(i as _, i as _, 0).expect("failed to map acpi table entry");
        }

        if !header.is_checksum_valid() {
            println!("Cannot parse ACPI table: the checksum is invalid!");
            return;
        }

        let sig = core::str::from_utf8(&header.signature).unwrap_or("ERR ");
        let a = header.length;
        let b = header.revision;

        println!(
            "Table: {} Length: {} Revision: {}",
            sig,
            a,
            b
        );
    }
}