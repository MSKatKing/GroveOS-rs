pushd uefi_loader
cargo build || exit /b
popd

pushd kernel
cargo build || exit /b
popd

mkdir esp\efi\boot
xcopy .\target\x86_64-unknown-uefi\debug\uefi_loader.efi .\esp\efi\boot\bootx64.efi /y
xcopy .\target\x86_64-unknown-groveos\debug\kernel .\esp\kernel.elf /y

qemu-system-x86_64 -machine q35 -drive if=pflash,format=raw,readonly=on,file=OVMF_CODE.fd -drive if=pflash,format=raw,readonly=on,file=OVMF_VARS.fd -drive format=raw,file=fat:rw:esp -d int,cpu_reset -D qemu.log