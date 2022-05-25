.altmacro
.macro SAVE_SN n
    sd s\n, (\n+2)*8(a0)
.endm
.macro LOAD_SN n
    ld s\n, (\n+2)*8(a1)
.endm
    .section .text
    .globl __switch
    # __switch(current_task_cx_ptr: *mut TaskContext, next_task_cx_ptr: *const TaskContext)
    # 调用__switch传入两个参数,分别通过a0、a1寄存器保存
    # a0: 当前任务A上下文
    # a1：即将被切换到的任务B的上下文
__switch:
    # 在A任务上下文空间里保存当前CPU寄存器的快照
    sd sp, 8(a0)
    sd ra, 0(a0)
    .set n, 0
    .rept 12
        SAVE_SN %n
        .set n, n+1
    .endr
    # 根据任务B保存的上下文内容来恢复ra寄存器、s0~s11寄存器以及sp寄存器
    ld ra, 0(a1)
    .set n, 0
    .rept 12
        LOAD_SN %n
        .set n, n + 1
    .endr
    ld sp, 8(a1)
    ret