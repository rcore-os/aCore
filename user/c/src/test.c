#include <unistd.h>

int main(int argc, char* argv[])
{
    for (int i = 0; i < argc; i++) {
        write(1, argv[i], 10);
        write(1, "\n", 1);
    }
}
