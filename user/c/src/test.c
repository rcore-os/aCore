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
}
