#include <asynccall.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

int main(int argc, char* argv[])
{
    puts("Hello, World!");
    char str[] = "PID: 0";
    str[5] = getpid() + 48;
    puts(str);
    sched_yield();

    for (int i = 0; i < argc; i++) {
        puts(argv[i]);
    }
    sched_yield();

    struct async_call_buffer buffer;
    async_call_buffer_init(16, 16, &buffer);
    for (int i = 0; i < 10; i++) {
        int cached_tail = *buffer.req_ring.ktail;
        struct request_ring_entry* req = request_ring_get_entry(&buffer, cached_tail);
        req->user_data = 0x1000 + i;
        char str[] = "Hello, async call!\n";
        async_call_write(req, stdout, str, strlen(str), 0);
        *buffer.req_ring.ktail = cached_tail + 1;
    }

    while (*buffer.comp_ring.ktail < 10) {
        __sync_synchronize();
        while (*buffer.comp_ring.khead < *buffer.comp_ring.ktail) {
            int cached_head = *buffer.comp_ring.khead;
            struct complete_ring_entry* comp = complete_ring_get_entry(&buffer, cached_head);
            if (comp->user_data != 0x1000 + cached_head) {
                return 1;
            }
            *buffer.comp_ring.khead = cached_head + 1;
        }
    }

    return 0;
}
