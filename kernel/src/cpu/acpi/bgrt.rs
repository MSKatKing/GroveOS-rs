use crate::cpu::acpi::AcpiSdtHeader;
use crate::mem::heap::PAGE_SIZE;
use crate::mem::page::page_table::PageTable;
use crate::mem::page::VirtAddr;
use crate::println;
use crate::screen::framebuffer_writer;

#[repr(C, packed)]
struct BGRTHeader {
    header: AcpiSdtHeader,
    version: u16,
    status: u8,
    image_type: u8,
    addr: u64,
    x_offset: u32,
    y_offset: u32,
}

#[repr(C, packed)]
struct BMPHeader {
    checksum: [u8; 2],
    file_size: u32,
    reserved: u32,
    pixel_data_offset: u32,
    dib_header_size: u32,
    width: u32,
    height: u32,
    planes: u16,
    bits_per_pixel: u16,
}

pub fn draw_img(header: &AcpiSdtHeader) {
    unsafe {
        let header = header as *const AcpiSdtHeader as *const BGRTHeader;
        let header = &*header;

        let img_ptr = header.addr;

        PageTable::current().map_addr(img_ptr, img_ptr, 0).expect("Failed to map image");

        let img_ptr = &*(img_ptr as *mut BMPHeader);
        if &img_ptr.checksum == b"BM" {
            let image_len = img_ptr.width * img_ptr.height;

            for i in 0..((image_len as usize * size_of::<u32>() + 0xFFF) / PAGE_SIZE) {
                PageTable::current().map_addr(
                    img_ptr as *const _ as VirtAddr + i as VirtAddr * PAGE_SIZE as VirtAddr,
                    img_ptr as *const _ as VirtAddr + i as VirtAddr * PAGE_SIZE as VirtAddr,
                    0
                ).expect("Failed to map image");
            }

            let image_data = header.addr + img_ptr.pixel_data_offset as u64;
            let image_data = image_data as *mut u8;

            let bytes_per_pixel = (img_ptr.bits_per_pixel / 8) as u32;
            let row_size = ((img_ptr.width * bytes_per_pixel + 3) / 4) * 4;

            let y_offset = framebuffer_writer().height as f32 * 0.2;
            let y_offset = y_offset as usize;

            for x in 0..img_ptr.width {
                for y in 0..img_ptr.height {
                    let inverted_y = img_ptr.height - 1 - y;
                    let pixel_offset = img_ptr.pixel_data_offset + (inverted_y * row_size) + (x * bytes_per_pixel);
                    let pixel = match bytes_per_pixel {
                        3 => {
                            let b = *image_data.offset(pixel_offset as isize) as u32;
                            let g = *image_data.offset(pixel_offset as isize + 1) as u32;
                            let r = *image_data.offset(pixel_offset as isize + 2) as u32;
                            (r << 16) | (g << 8) | b
                        },
                        4 => {
                            let b = *image_data.offset(pixel_offset as isize) as u32;
                            let g = *image_data.offset(pixel_offset as isize + 1) as u32;
                            let r = *image_data.offset(pixel_offset as isize + 2) as u32;
                            let a = *image_data.offset(pixel_offset as isize + 3) as u32;
                            (a << 24) | (r << 16) | (g << 8) | b
                        },
                        _ => 0
                    };
                    framebuffer_writer().force_write((x + header.x_offset) as usize, y as usize + y_offset, pixel)
                }
            }
        }
    }
}