.section .text.entry
.global _start
_start:
    la sp, boot_stack_top
    call rust_main

.section .bss.stack
.global boot_stack
.global boot_stack_top
boot_stack:
    .space 4096 * 16
boot_stack_top:
