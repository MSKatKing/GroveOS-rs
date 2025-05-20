use core::arch::{asm, naked_asm};

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct GDTEntry {
    limit: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
}

impl GDTEntry {
    pub const fn empty() -> Self {
        Self {
            limit: 0,
            base_low: 0,
            base_middle: 0,
            access: 0,
            granularity: 0,
            base_high: 0,
        }
    }
    
    pub fn new(base: u32, limit: u32, access: u8, granularity: u8) -> Self {
        Self {
            limit: limit as _,
            base_low: (base & 0xFFFF) as _,
            base_middle: ((base >> 16) & 0xFF) as _,
            base_high: ((base >> 24) & 0xFF) as _,
            access,
            granularity: granularity << 4 | ((limit >> 16) & 0x0F) as u8,
        }
    }
}

const GDT_ENTRIES: usize = 5;
static mut GDT: [GDTEntry; GDT_ENTRIES] = [GDTEntry::empty(); GDT_ENTRIES];

pub fn install_gdt_defaults() {
    unsafe {
        GDT[1] = GDTEntry::new(0, 0xFFFF, 0x9A, 0xA);
        GDT[2] = GDTEntry::new(0, 0xFFFF, 0x92, 0xC);
        GDT[3] = GDTEntry::new(0, 0xFFFF, 0xFA, 0xA);
        GDT[4] = GDTEntry::new(0, 0xFFFF, 0xF2, 0xC);
    }
}

#[unsafe(naked)]
pub extern "C" fn lgdt() {
    #[repr(packed)]
    struct GDTDescriptor {
        limit: u16,
        base: u64,
    }

    static mut GDTP: GDTDescriptor = GDTDescriptor {
        limit: GDT_ENTRIES as u16 * size_of::<GDTEntry>() as u16 - 1,
        base: 0,
    };
    
    #[unsafe(no_mangle)]
    fn lgdt_inner() {
        unsafe {
            GDTP.base = &raw const GDT as u64;

            asm!("cli");
            asm!("lgdt [{gdtp}]", gdtp = in(reg) &raw const GDTP, options(nostack))
        }
    }

    naked_asm!(
        "call lgdt_inner",
        "mov ax, 0x10",
        "mov ds, ax",
        "mov es, ax",
        "mov fs, ax",
        "mov gs, ax",
        "mov ss, ax",
        "pop rdi",
        "mov rax, 0x08",
        "push rax",
        "push rdi",
        "retfq",
    );
}