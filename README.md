# GroveOS

[![Rust](https://github.com/MSKatKing/GroveOS-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/MSKatKing/GroveOS-rs/actions/workflows/rust.yml)

Welcome to the repository of GroveOS!

### What is GroveOS?

GroveOS is a Rust-based operating system built from scratch mostly as a hobby. This codebase is accompanied by a collection of blog-style pages explaining a lot of what the code does. If anyone is looking at building their own operating system, I hope that those pages prove to be helpful. The pages can be found [here](https://dinoslice.com/grove-os/).

### Roadmap

- [x] UEFI bootloader launching ELF kernel
- [ ] Basic CPU memory structures setup (GDT, IDT, PML4, etc...)
- [ ] Heap memory management
- [ ] Simple file system operations
- [ ] Launching user processes
- [ ] Basic libc implementation
- [ ] Process scheduler
- [ ] Simple shell program
- [ ] Custom dynamic linker
- [ ] Kernel drivers
- [ ] More advanced kernel API / libc
- [ ] Kernel self-updater
- [ ] Support PE executables and libraries (exe/dll)

### Thanks

Special thanks so Scott Fial for the [Tamsyn](http://www.fial.com/~scott/tamsyn-font/) font used in the kernel!