#include <asynccall.h>
#include <stdio.h>
#include <unistd.h>

int main(int argc, char* argv[])
{
    puts("Hello, World!");
    char str[] = "PID: 0";
    str[5] = getpid() + 48;
    puts(str);
    for (int i = 0; i < argc; i++) {
        puts(argv[i]);
    }

    struct async_call_info info;
    setup_async_call(0, 0, 0, &info);
    for (int i = 0; i < info.buf_size + 5; i++)
        putchar(info.user_buf_ptr->data[i]);
}
