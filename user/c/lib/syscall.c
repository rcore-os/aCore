#include <stddef.h>
#include <unistd.h>

#include "syscall.h"

ssize_t write(int fd, const void* buf, size_t count)
{
    return syscall(SYS_write, fd, buf, count);
}

int getpid(void)
{
    return syscall(SYS_getpid);
}

int sched_yield(void)
{
    return syscall(SYS_sched_yield);
}

void exit(int code)
{
    syscall(SYS_exit, code);
}

void setup_async_call(int arg0, int arg1, uint64_t flags, void* info)
{
    syscall(SYS_setup_async_call, arg0, arg1, flags, info);
}
