#![no_main]
#![no_std]

use log::info;
use uefi::prelude::*;
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::file::{File, FileAttribute, FileHandle, FileMode};
use uefi::proto::media::fs::SimpleFileSystem;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    let _kernel = load_kernel().expect("Failed to load the kernel");


    info!("Hello world!");
    loop {}
}

fn load_kernel() -> Option<FileHandle> {
    let image = boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle()).ok()?;
    let mut fs = boot::open_protocol_exclusive::<SimpleFileSystem>(image.device()?).ok()?;

    let mut directory = fs.open_volume().ok()?;
    
    directory.open(cstr16!("kernel.elf"), FileMode::Read, FileAttribute::empty()).ok()
}