# 构建项目
cargo build --release

# 裁剪内核文件，删除元数据信息，使得内核文件被qemu加载到0x80200000处的时候刚好是程序入口
rust-objcopy --strip-all ./target/riscv64gc-unknown-none-elf/release/rcore-kernel -O binary ./target/riscv64gc-unknown-none-elf/release/rcore-kernel.bin

# 通过qemu运行
# 介绍参考： http://rcore-os.cn/rCore-Tutorial-Book-v3/chapter1/4first-instruction-in-kernel2.html#gdb
qemu-system-riscv64 -machine virt -nographic -bios ../bootloader/rustsbi-qemu.bin -device loader,file=target/riscv64gc-unknown-none-elf/release/rcore-kernel.bin,addr=0x80200000