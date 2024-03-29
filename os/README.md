## 操作系统结构

* .cargo: 项目配置文件，其中配置了编译的目标为"riscv64gc-unknown-none-elf"，同时指定了链接脚本为linker.ld文件
* linker.ld: 链接脚本，通过链接脚本 (Linker Script) 调整链接器的行为，使得最终生成的可执行文件的内存布局符合Qemu的预期，即内核第一条指令的地址应该位于 0x80200000 
    * 脚本介绍参考: [调整内核的内存布局](http://rcore-os.cn/rCore-Tutorial-Book-v3/chapter1/4first-instruction-in-kernel2.html#id4)
* entry.asm: 程序结构分配脚本。功能包括：分配启动栈空间。
    * 介绍参考: [分配并使用启动栈](http://rcore-os.cn/rCore-Tutorial-Book-v3/chapter1/5support-func-call.html#jump-practice)
* loader.rs: 对APP应用的加载,将所有应用都加载到内存中
* sync/up.rs: 在RefCell的基础上再进行封装的UPSafeCell，允许我们在单核上安全使用可变全局变量
* trap: 特权级的切换模块
* task/switch.S: 控制流切换对寄存器的操作
* timer.rs: 时钟中断相关模块