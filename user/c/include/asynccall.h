#ifndef __ASYNC_CALL_H__
#define __ASYNC_CALL_H__

#include <stddef.h>

struct async_call_buffer {
    uint8_t data[233];
};

struct async_call_info {
    struct async_call_buffer* user_buf_ptr;
    size_t buf_size;
};

void setup_async_call(int arg0, int arg1, uint64_t flags, struct async_call_info* info);

#endif // __ASYNC_CALL_H__
