pushd uefi_loader
cargo build --target x86_64-unknown-uefi || exit
popd

pushd kernel
cargo build --target x86_64-unknown-none || exit
popd

mkdir -p esp/efi/boot
cp target/x86_64-unknown-uefi/debug/uefi_loader.efi ./esp/efi/boot/bootx64.efi
cp target/x86_64-unknown-none/debug/kernel ./esp/kernel.elf

qemu-system-x86_64 -drive if=pflash,format=raw,readonly=on,file=OVMF_CODE.fd -drive if=pflash,format=raw,readonly=on,file=OVMF_VARS.fd -drive format=raw,file=fat:rw:esp -d int,cpu_reset -D qemu.log