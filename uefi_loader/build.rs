use std::fs;
use std::fs::DirBuilder;

fn main() {
    DirBuilder::new().recursive(true).create("../esp/efi/boot").expect("Failed to create image path");
    fs::copy("target/x86_64-unknown-uefi/debug/uefi_loader.efi", "../esp/efi/boot/bootx64.efi").expect("Failed to move uefi_loader.efi");
}