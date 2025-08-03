use crate::println;
use core::arch::asm;
use core::cell::LazyCell;

pub mod gdt;
pub mod idt;

pub fn print_cpu_info() {
    println!("---------- CPU Info ----------");
    println!("CPU Brand: {}", cpu_brand_string());
    println!("CPU Vendor: {}", cpu_vendor_string());
    println!("Cores: {}", get_cores_per_socket());
    println!("Logical Cores: {}", get_num_logical_processors());
    println!("Supports Virtualization: {}", supports_virtualization());
    println!("------------------------------");
}

#[inline(always)]
pub fn cpuid(leaf: u32, subleaf: u32) -> (u32, u32, u32, u32) {
    let eax: u32;
    let ebx: u32;
    let ecx: u32;
    let edx: u32;

    unsafe {
        asm!(
            "cpuid",
            inlateout("eax") leaf => eax,
            inlateout("ecx") subleaf => ecx,
            lateout("edx") edx,
        );

        asm!(
            "mov {:e}, ebx",
            out(reg) ebx,
        )
    }

    (eax, ebx, ecx, edx)
}

pub fn cpu_brand_string() -> &'static str {
    static mut BRAND: [u8; 49] = [0; 49];

    #[allow(static_mut_refs)]
    unsafe {
        BRAND.fill(0);
    }

    for i in 0..3 {
        let (eax, ebx, ecx, edx) = cpuid(0x80000002 + i as u32, 0);
        unsafe {
            BRAND[i * 16 + 0..i * 16 + 4].copy_from_slice(&eax.to_le_bytes());
            BRAND[i * 16 + 4..i * 16 + 8].copy_from_slice(&ebx.to_le_bytes());
            BRAND[i * 16 + 8..i * 16 + 12].copy_from_slice(&ecx.to_le_bytes());
            BRAND[i * 16 + 12..i * 16 + 16].copy_from_slice(&edx.to_le_bytes());

        }
    }

    #[allow(static_mut_refs)]
    unsafe {
        let len = BRAND.iter().position(|b| *b == 0).unwrap_or(48);
        str::from_utf8_unchecked(&BRAND[..len])
    }
}

pub fn cpu_vendor_string() -> &'static str {
    static mut VENDOR: [u8; 12] = [0; 12];

    let (_, ebx, ecx, edx) = cpuid(0, 0);
    unsafe {
        VENDOR[0..4].copy_from_slice(&ebx.to_le_bytes());
        VENDOR[4..8].copy_from_slice(&edx.to_le_bytes());
        VENDOR[8..12].copy_from_slice(&ecx.to_le_bytes());
    }

    #[allow(static_mut_refs)]
    unsafe {
        str::from_utf8_unchecked(&VENDOR)
    }
}

pub fn get_num_logical_processors() -> u32 {
    match *CPU_VENDOR {
        CPUVendor::Intel => {
            let mut logical_processors = 0;
            let mut level = 0;
            loop {
                let (_, ebx, ecx, _) = cpuid(0x0B, level);
                if (ecx & 0xFF) == 0 {
                    break;
                }
                if (ecx & 0xFF) == 1 {
                    logical_processors = ebx & 0xFFFF;
                    break;
                }
                level += 1;
            }
            if logical_processors == 0 {
                let (_, ebx, _, _) = cpuid(1, 0);
                (ebx >> 16) & 0xFF
            } else {
                logical_processors
            }
        },
        CPUVendor::AMD | CPUVendor::Other(_) => {
            let (_, ebx, _, _) = cpuid(1, 0);
            (ebx >> 16) & 0xFF
        },
    }
}

pub fn get_cores_per_socket() -> u32 {
    match *CPU_VENDOR {
        CPUVendor::Intel => {
            let mut cores = 0;
            let mut level = 0;
            loop {
                let (_, ebx, ecx, _) = cpuid(0x0B, level);
                if (ecx & 0xFF) == 0 {
                    break;
                }
                if (ecx & 0xFF) == 2 {
                    cores = ebx & 0xFFFF;
                    break;
                }
                level += 1;
            }
            if cores == 0 {
                let (eax, _, _, _) = cpuid(1, 0);
                ((eax >> 26) & 0x3F) + 1
            } else {
                cores
            }
        },
        CPUVendor::AMD | CPUVendor::Other(_) => {
            let (eax, _, _, _) = cpuid(1, 0);
            ((eax >> 26) & 0x3F) + 1
        },
    }
}

pub fn supports_virtualization() -> bool {
    let (_, _, ecx, _) = cpuid(1, 0);
    let has_vmx = (ecx & (1 << 5)) != 0;
    let has_svm = {
        let (_, _, ecx, _) = cpuid(0x80000001, 0);
        (ecx & (1 << 2)) != 0
    };

    has_vmx || has_svm
}

pub enum CPUVendor {
    Intel,
    AMD,
    Other(&'static str),
}

pub const CPU_VENDOR: LazyCell<CPUVendor> = LazyCell::new(||
    match cpu_vendor_string() {
        "GenuineIntel" => CPUVendor::Intel,
        "AuthenticAMD" => CPUVendor::AMD,
        other => CPUVendor::Other(other)
    }
);