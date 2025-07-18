use core::arch::asm;

type ISR = unsafe extern "C" fn();

#[repr(packed)]
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub struct IDTEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    zero: u32,
}

impl IDTEntry {
    pub const fn empty() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            zero: 0,
        }
    }

    pub fn new(isr: ISR, flags: Option<u8>) -> Self {
        let isr = isr as *const () as u64;

        Self {
            offset_low: (isr & 0xFFFF) as _,
            offset_mid: ((isr >> 16) & 0xFFFF) as _,
            offset_high: ((isr >> 32) & 0xFFFFFFFF) as _,
            selector: 0x08,
            ist: 0x00,
            type_attr: flags.unwrap_or(0x8E),
            zero: 0,
        }
    }
}

const IDT_ENTRIES: usize = 256;
static mut IDT: [IDTEntry; IDT_ENTRIES] = [IDTEntry::empty(); IDT_ENTRIES];

pub fn set_idt_entry(entry: IDTEntry, index: usize) {
    if index >= IDT_ENTRIES {
        return;
    }
    unsafe {
        IDT[index] = entry;
    }
}

pub fn lidt() {
    #[repr(packed)]
    struct IDTPointer {
        limit: u16,
        base: u64,
    }

    static mut IDTP: IDTPointer = IDTPointer {
        limit: IDT_ENTRIES as u16 * size_of::<IDTEntry>() as u16 - 1,
        base: 0,
    };

    unsafe {
        IDTP.base = &raw const IDT as u64;

        asm!("lidt [{idtp}]", idtp = in(reg) &raw const IDTP, options(nostack, preserves_flags));
        asm!("sti")
    }
}
