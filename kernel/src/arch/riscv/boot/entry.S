    .section .text.entry
    .global _start
_start:
    # a0 == hartid
    # pc == 0x80200000
    mv tp, a0

    add t0, a0, 1
    slli t0, t0, 16
    la sp, boot_stack
    add sp, sp, t0

    tail start_kernel

    .section .bss.stack
    .global boot_stack
    .global boot_stack_top
boot_stack:
    .space 64 * 1024 * 2    # 64 K/cores * 2
boot_stack_top:
