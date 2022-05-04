cargo build --release
rust-objcopy --strip-all ./target/riscv64gc-unknown-none-elf/release/rcore-kernel -O binary ./target/riscv64gc-unknown-none-elf/release/rcore-kernel.bin
qemu-system-riscv64 -machine virt -nographic -bios bootloader/rustsbi-qemu.bin -device loader,file=target/riscv64gc-unknown-none-elf/release/rcore-kernel.bin,addr=0x80200000 