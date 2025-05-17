OUTPUT_FORMAT(elf64-x86-64)
ENTRY(_start)

KERNEL_OFFSET = 0xFFFFFFFF80000000;

SECTIONS
{
    . = KERNEL_OFFSET;

    __kernel_vstart = .;

    .text : AT(ADDR(.text) - KERNEL_OFFSET) {
        *(.multiboot)
        *(.text*)
        . = ALIGN(0x1000);
    }

    .rodata : AT(ADDR(.rodata) - KERNEL_OFFSET) {
        *(.rodata*)
        . = ALIGN(0x1000);
    }

    .data : AT(ADDR(.data) - KERNEL_OFFSET) {
        *(.data*)
        . = ALIGN(0x1000);
    }

    .eh_frame : AT(ADDR(.eh_frame) - KERNEL_OFFSET) {
        *(.eh_frame)
        . = ALIGN(0x1000);
    }

    .bss : AT(ADDR(.bss) - KERNEL_OFFSET) {
        *(.bss*)
        *(COMMON)
        . = ALIGN(0x1000);
    }

    __kernel_vend = .;

    /DISCARD/ : {
        *(.comment)
        *(note.*)
    }
}