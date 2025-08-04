mod mcfg;
mod bgrt;
pub mod apic;

use crate::cpu::acpi::apic::APIC_SYSTEM;
use crate::mem::heap::PAGE_SIZE;
use crate::mem::page::page_table::PageTable;
use crate::mem::page::VirtAddr;
use crate::println;
use alloc::vec::Vec;
use core::ptr;

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
    #[allow(static_mut_refs)]
    for system in unsafe { &mut ACPI_SYSTEMS } {
        system.preinit()
    }

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

        if rsdp.revision >= 2 && rsdp.length as usize >= size_of::<Rsdp>() {
            let a = rsdp.xsdt_address;

            PageTable::current().map_addr(a, a, 0).expect("failed to map xsdt");

            parse_xsdt(rsdp.xsdt_address as *const AcpiSdtHeader);
        }
    }

    #[allow(static_mut_refs)]
    for system in unsafe { &mut ACPI_SYSTEMS } {
        if !system.loaded() {
            println!("Couldn't load system for ACPI table {}", system.targeted_table());
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
            return;
        }

        let entry_count = (header.length as usize - size_of::<AcpiSdtHeader>()) / 8;
        let entry_ptr = xsdt_ptr.cast::<u8>().offset(size_of::<AcpiSdtHeader>() as isize).cast::<u64>();

        PageTable::current().map_addr(entry_ptr as VirtAddr, entry_ptr as VirtAddr, 0).expect("failed to map xsdt entry");

        for i in 0..entry_count {
            let entry = ptr::read_unaligned(entry_ptr.offset(i as isize)) as *const AcpiSdtHeader;

            if entry.is_null() {
                continue;
            }

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
            return;
        }

        let sig = core::str::from_utf8(&header.signature).unwrap_or("ERR ");

        #[allow(static_mut_refs)]
        for system in &mut ACPI_SYSTEMS {
            if sig == system.targeted_table() {
                match system.init(header) {
                    Ok(()) => {},
                    Err(msg) => println!("An error occurred while initializing an ACPI system for table {}: {}", sig, msg),
                }
            }
        }
    }
}

pub trait AcpiInitializable: Send + Sync {
    fn preinit(&mut self);
    fn init(&mut self, header: &AcpiSdtHeader) -> Result<(), &'static str>;
    fn targeted_table(&self) -> &'static str;
    fn loaded(&self) -> bool;
}

static mut ACPI_SYSTEMS: Vec<&'static mut dyn AcpiInitializable> = Vec::new();

pub fn register_acpi_system(system: &'static mut dyn AcpiInitializable) {
    #[allow(static_mut_refs)]
    unsafe {
        ACPI_SYSTEMS.push(system);
    }
}

pub fn register_default_systems() {
    #[allow(static_mut_refs)]
    register_acpi_system(unsafe { &mut APIC_SYSTEM });
}