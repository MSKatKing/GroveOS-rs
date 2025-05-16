pushd uefi_loader
cargo build --target x86_64-unknown-uefi
popd

mkdir esp\efi\boot
xcopy .\target\x86_64-unknown-uefi\debug\uefi_loader.efi .\esp\efi\boot\bootx64.efi /y

qemu-system-x86_64 -drive if=pflash,format=raw,readonly=on,file=OVMF_CODE.fd -drive if=pflash,format=raw,readonly=on,file=OVMF_VARS.fd -drive format=raw,file=fat:rw:esp