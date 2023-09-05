.altmacro
.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm
    .section .text.trampoline
    .globl __alltraps
    .globl __restore
    # 将__alltraps地址4字节对齐
    .align 2
__alltraps:
    # sscratch 指向应用地址空间，因此后面实际上是在应用地址空间保存上下文
    csrrw sp, sscratch, sp
    # 保存trap上下文的通用寄存器x0~x31(跳过x0和tp(x4))
    sd x1, 1*8(sp)
    sd x3, 3*8(sp)
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    # 将CSR sstatus和sepc的值分别读到寄存器t0和t1中然后保存到内核栈对应的位置上
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)
    # 将sscratch的值读到寄存器t2并保存在内核栈上(此时sscratch指向用户栈)
    csrr t2, sscratch
    sd t2, 2*8(sp)
    # 将内核栈地址(token)加载到t0
    ld t0, 34*8(sp)
    # 将trap handler入口地址加载到t1
    ld t1, 36*8(sp)
    # 将sp修改为应用内核栈顶的地址
    ld sp, 35*8(sp)
    # 将 satp 修改为内核地址空间的 token 并使用 sfence.vma 刷新快表，这就切换到了内核地址空间；
    csrw satp, t0
    sfence.vma
    # 跳转到trap handler(t1)
    jr t1

# 传入两个参数:a0: trap上下文在应用地址空间的位置, a1: 即将回到的应用的地址空间
# trap上下文就是应用上下文保存的地址空间
__restore:
    # 切换回应用地址空间
    csrw satp, a1
    sfence.vma
    # trap handler上下文保存在sscratch寄存器中
    csrw sscratch, a0
    # 将sp修改为trap上下文的位置, 后面基于它恢复各通用寄存器和 CSR
    mv sp, a0
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    # restore general-purpuse registers except sp/tp
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    ld sp, 2*8(sp)
    # 返回用户态
    sret