.text
.globl _start
_start:
    mv a0, sp
    tail __start_main
