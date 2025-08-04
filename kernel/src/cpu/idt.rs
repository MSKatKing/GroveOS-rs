use core::arch::asm;

type ISR = unsafe extern "x86-interrupt" fn(*mut ());
type ISR_ERR = unsafe extern "x86-interrupt" fn(*mut (), u64);

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

    pub fn new_error(isr: ISR_ERR, flags: Option<u8>) -> Self {
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
        IDTP.limit = IDT_ENTRIES as u16 * size_of::<IDTEntry>() as u16 - 1;
        IDTP.base = &raw const IDT as u64;

        asm!("lidt [{idtp}]", idtp = in(reg) &raw const IDTP, options(nostack, preserves_flags));
        asm!("sti")
    }
}

pub fn setup_idt() {
    use IDTEntry as E;

    set_idt_entry(E::new(divide_error, None), 0);              // #DE
    set_idt_entry(E::new(debug, None), 1);                     // #DB
    set_idt_entry(E::new(non_maskable, None), 2);              // NMI
    set_idt_entry(E::new(breakpoint, None), 3);                // #BP
    set_idt_entry(E::new(overflow, None), 4);                  // #OF
    set_idt_entry(E::new(bound_range, None), 5);               // #BR
    set_idt_entry(E::new(invalid_opcode, None), 6);            // #UD
    set_idt_entry(E::new(device_not_available, None), 7);      // #NM
    set_idt_entry(E::new(double_fault, None), 8);              // #DF
    // Skipping 9 (obsolete: Coprocessor Segment Overrun)
    set_idt_entry(E::new(invalid_tss, None), 10);              // #TS
    set_idt_entry(E::new(segment_not_present, None), 11);      // #NP
    set_idt_entry(E::new(stack_segment_fault, None), 12);      // #SS
    set_idt_entry(E::new(general_protection_fault, None), 13); // #GP
    set_idt_entry(E::new_error(page_fault, None), 14);         // #PF
    set_idt_entry(E::new(x87_floating_point, None), 16);       // #MF
    set_idt_entry(E::new(alignment_check, None), 17);          // #AC
    set_idt_entry(E::new(machine_check, None), 18);            // #MC
    set_idt_entry(E::new(simd, None), 19);                     // #XM
    set_idt_entry(E::new(virtualization, None), 20);           // #VE
    set_idt_entry(E::new(security_exception, None), 30);       // #CP
}

macro_rules! isr {
    ($interrupt:ident) => {
        #[inline]
        pub extern "x86-interrupt" fn $interrupt(_stack_frame: *mut ()) {
            crate::println!("{} interrupt", stringify!($interrupt));

            loop {
                unsafe {
                    asm!("hlt")
                }
            }
        }
    };
}

isr!(divide_error);
isr!(debug);
isr!(non_maskable);
isr!(breakpoint);
isr!(overflow);
isr!(bound_range);
isr!(invalid_opcode);
isr!(device_not_available);
isr!(double_fault);
isr!(invalid_tss);
isr!(segment_not_present);
isr!(stack_segment_fault);
isr!(general_protection_fault);

pub extern "x86-interrupt" fn page_fault(_stack_frame: *mut (), error_code: u64) {
    crate::println!("page fault");

    let cr2: u64;
    unsafe { asm!("mov {}, cr2", out(reg) cr2, options(nostack, preserves_flags)) }

    crate::println!("faulting address {:#x}", cr2);
    crate::println!("errorcode: {:#x}", error_code);

    loop {
        unsafe { asm!("hlt") }
    }
}

isr!(x87_floating_point);
isr!(alignment_check);
isr!(machine_check);
isr!(simd);
isr!(virtualization);
isr!(security_exception);
