#include <stddef.h>
#include <unistd.h>

#include "syscall.h"

ssize_t write(int fd, const void* buf, size_t count)
{
    return syscall(SYS_write, fd, buf, count);
}

void exit(int code)
{
    syscall(SYS_exit, code);
}

extern int main(int, char**);

int __start_main(long* p)
{
    int argc = p[0];
    char** argv = (void*)(p + 1);
    exit(main(argc, argv));
    return 0;
}
