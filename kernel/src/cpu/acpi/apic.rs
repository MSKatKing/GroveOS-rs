use alloc::vec::Vec;
use crate::cpu::acpi::{AcpiInitializable, AcpiSdtHeader};
use crate::{print, println};

pub static mut APIC_SYSTEM: ApicTables = ApicTables { loaded: false, processor_apics: Vec::new(), };

pub struct ProcessorLocalApic {
    id: u8,
    apic_id: u8,
    enabled: bool,
}

pub struct ApicTables {
    pub loaded: bool,
    pub processor_apics: Vec<ProcessorLocalApic>
}

#[repr(C, packed)]
struct MadtTable {
    header: AcpiSdtHeader,
    apic_addr: u32,
    flags: u32,
}

#[repr(C, packed)]
struct ApicRecordHeader {
    entry_type: u8,
    record_length: u8,
}

impl AcpiInitializable for ApicTables {
    fn preinit(&mut self) {
        self.loaded = false;
        self.processor_apics = Vec::new();
    }

    fn init(&mut self, header: &AcpiSdtHeader) -> Result<(), &'static str> {
        unsafe {
            let header = &*(header as *const AcpiSdtHeader).cast::<MadtTable>();

            let start = header as *const _ as *const u8;
            let mut traversed = size_of::<MadtTable>() as u32;
            while header.header.length > traversed {
                print!("@ {} ", traversed);
                let record_header = &*start.add(traversed as usize).cast::<ApicRecordHeader>();
                traversed += (record_header.record_length as u32).max(size_of::<ApicRecordHeader>() as u32);

                match record_header.entry_type {
                    0 => {
                        let record_header = convert_ref::<ApicRecordHeader, ProcessorLocalApicRaw>(record_header);

                        if record_header.is_enabled() || record_header.online_capable() {
                            self.processor_apics.push(ProcessorLocalApic {
                                id: record_header.acpi_processor_id,
                                apic_id: record_header.apic_id,
                                enabled: record_header.is_enabled()
                            });
                        } else {
                            println!("Skipping processor {} because it cannot be used.", record_header.acpi_processor_id);
                        }
                    },
                    1 => println!("i/o apic found"),
                    2 => println!("i/o apic interrupt source override found"),
                    3 => println!("i/o apic non-maskable interrupt source found"),
                    4 => println!("local apic non-maskable interrupt found"),
                    5 => println!("local apic address override"),
                    9 => println!("processor local x2apic found"),
                    id => println!("unknown apic entry type: {}", id),
                }
            }
        }

        self.loaded = true;
        Ok(())
    }

    fn targeted_table(&self) -> &'static str {
        "APIC"
    }

    fn loaded(&self) -> bool {
        self.loaded
    }
}

#[inline(always)]
unsafe fn convert_ref<T, U>(ptr: &T) -> &U {
    unsafe { (ptr as *const T as *const U).as_ref_unchecked() }
}

#[repr(C, packed)]
struct ProcessorLocalApicRaw {
    header: ApicRecordHeader,
    acpi_processor_id: u8,
    apic_id: u8,
    flags: u32,
}

impl ProcessorLocalApicRaw {
    fn is_enabled(&self) -> bool {
        self.flags & 0x01 == 0x01
    }

    fn online_capable(&self) -> bool {
        self.flags & 0x02 == 0x02
    }
}

#[repr(C, packed)]
struct IOApicRaw {
    header: ApicRecordHeader,
    apic_id: u8,
    _reserved: u8,
    apic_addr: u32,
    interrupt_base: u32,
}

#[repr(C, packed)]
struct IOApicInterruptSourceOverrideRaw {
    header: ApicRecordHeader,
    bus_source: u8,
    irq_source: u8,
    interrupt: u32,
    flags: u16,
}

#[repr(C, packed)]
struct IOApicNonMaskableInterruptSourceRaw {
    header: ApicRecordHeader,
    nmi_source: u8,
    _reserved: u8,
    flags: u16,
    interrupt: u32,
}

#[repr(C, packed)]
struct LocalApicNonMaskableInterruptsRaw {
    header: ApicRecordHeader,
    processor_id: u8,
    flags: u16,
    lint_num: u8,
}

#[repr(C, packed)]
struct LocalApicAddressOverrideRaw {
    header: ApicRecordHeader,
    _reserved: u16,
    apic_addr: u64,
}

#[repr(C, packed)]
struct ProcessorLocalX2ApicRaw {
    header: ApicRecordHeader,
    _reserved: u16,
    apic_id: u32,
    flags: u32,
    acpi_id: u32,
}