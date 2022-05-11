.altmacro
.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm
    .section .text
    .globl __alltraps
    .globl __restore
    # 将__alltraps地址4字节对齐
    .align 2
__alltraps:
    # 在这之前sp指向用户栈,sscratch指向内核栈.现在两者互换
    csrrw sp, sscratch, sp
    # 准备在内核栈上保存上下文,预先分配34*8的栈帧,这里是对sp的修改,此时发生在内核栈上
    addi sp, sp, -34*8
    # 保存trap上下文的通用寄存器x0~x31(跳过x0和tp(x4))
    # save general-purpose registers
    sd x1, 1*8(sp)
    # skip sp(x2), we will save it later
    sd x3, 3*8(sp)
    # skip tp(x4), application does not use it
    # save x5~x31
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    # 将CSR sstatus和sepc的值分别读到寄存器t0和t1中然后保存到内核栈对应的位置上
    # we can use t0/t1/t2 freely, because they were saved on kernel stack
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)
    # 将sscratch的值读到寄存器t2并保存在内核栈上(此时sscratch指向用户栈)
    csrr t2, sscratch
    sd t2, 2*8(sp)
    # 让寄存器a0指向sp(刚刚保存的内核栈的Trap上下文地址)
    mv a0, sp
    # 调用trap_handler
    call trap_handler

__restore:
    # case1: start running app by __restore
    # case2: back to U after handling trap
    mv sp, a0
    # now sp->kernel stack(after allocated), sscratch->user stack
    # restore sstatus/sepc
    # 负责从内核栈栈顶的trap上下文中恢复通用寄存器和CSR
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    ld t2, 2*8(sp)
    # 需要先恢复三个CSR再恢复通用寄存器,这样前面三个临时寄存器才有用
    csrw sstatus, t0
    csrw sepc, t1
    csrw sscratch, t2
    # restore general-purpuse registers except sp/tp
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # release TrapContext on kernel stack
    # 回收内核栈上Trap上下文的内存
    addi sp, sp, 34*8
    # now sp->kernel stack, sscratch->user stack
    # 交换sscratch和sp,现在sp重新指向用户栈,sscratch指向内核栈
    csrrw sp, sscratch, sp
    # 返回U特权运行程序控制流
    sret