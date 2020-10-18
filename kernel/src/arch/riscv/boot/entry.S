    .section .text.entry
    .global _start
_start:
    # a0 == hartid
    # pc == 0x80200000
    mv      tp, a0

    # 1. set sp
    add     t0, a0, 1
    slli    t0, t0, 16
    la      sp, boot_stack
    add     sp, sp, t0

    # 2. enable paging
    # satp = (8 << 60) | PPN(boot_page_table_sv39)
    la      t0, boot_page_table_sv39
    srli    t0, t0, 12
    li      t1, 8 << 60
    or      t0, t0, t1
    csrw    satp, t0
    sfence.vma

    # 3. jump to start_kernel
    la      t0, start_kernel
    li      t1, 0xffffffff00000000
    add     t0, t0, t1
    add     sp, sp, t1
    jr      t0

    .section .bss.stack
    .global boot_stack
    .global boot_stack_top
boot_stack:
    .space 64 * 1024 * 2    # 64 K/cores * 2
boot_stack_top:

    .section .data
    .align 12
boot_page_table_sv39:
    .quad 0
    .quad 0
    # 0x00000000_80000000 -> 0x80000000 (1G, VRWXAD)
    .quad (0x80000 << 10) | 0xcf
    .zero 8 * 507
    # 0xffffffff_80000000 -> 0x80000000 (1G, VRWXAD)
    .quad (0x80000 << 10) | 0xcf
    .quad 0
