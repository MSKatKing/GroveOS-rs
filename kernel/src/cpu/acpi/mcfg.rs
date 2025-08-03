use core::slice;
use crate::cpu::acpi::AcpiSdtHeader;
use crate::mem::page::page_table::PageTable;
use crate::mem::page::{PhysAddr, VirtAddr};

const MCFG_RESERVED_SIZE: usize = 8;

#[repr(C, packed)]
#[derive(Debug)]
pub struct McfgAllocation {
    base_address: u64,
    pci_segment_group_number: u16,
    start_bus_number: u8,
    end_bus_number: u8,
    reserved: u32,
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct PCIConfigHeader {
    pub vendor_id: u16,
    pub device_id: u16,
    pub command: u16,
    pub status: u16,
    pub revision: u8,
    pub prog_if: u8,
    pub subclass: u8,
    pub class_code: u8,
    pub cache_line_size: u8,
    pub latency_timer: u8,
    pub header_type: u8,
    pub bist: u8,
}

pub fn parse_mcfg_table(header: &AcpiSdtHeader) -> Option<&[McfgAllocation]> {
    unsafe {
        let len = header.length as usize;

        let body_ptr = (header as *const _ as *const u8).add(size_of::<AcpiSdtHeader>() + MCFG_RESERVED_SIZE);
        let body_len = (len - size_of::<AcpiSdtHeader>()) - MCFG_RESERVED_SIZE;
        let entry_size = size_of::<McfgAllocation>();

        if body_len % entry_size != 0 {
            return None;
        }

        let entry_count = body_len / entry_size;
        Some(slice::from_raw_parts(body_ptr as *const McfgAllocation, entry_count))
    }
}

pub struct PciConfigIter {
    base_address: usize,
    bus: u8,
    device: u8,
    function: u8,
    start_bus: u8,
    end_bus: u8,
}

impl Iterator for PciConfigIter {
    type Item = *const PCIConfigHeader;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bus >= self.end_bus {
            return None;
        }

        let addr = self.base_address + ((self.bus as usize) << 20)
        + ((self.device as usize) << 15)
        + ((self.function as usize) << 12);

        self.function += 1;
        if self.function > 7 {
            self.function = 0;
            self.device += 1;
            if self.device > 32 {
                self.device = 0;

                if self.bus == u8::MAX {
                    return None;
                }

                self.bus += 1;
            }
        }

        PageTable::current().map_addr(addr as VirtAddr, addr as PhysAddr, 0).ok()?;

        Some(addr as *const PCIConfigHeader)
    }
}

impl McfgAllocation {
    pub fn iter(&self) -> PciConfigIter {
        PciConfigIter {
            base_address: self.base_address as _,
            bus: self.start_bus_number,
            device: 0,
            function: 0,
            start_bus: self.start_bus_number,
            end_bus: self.end_bus_number,
        }
    }
}