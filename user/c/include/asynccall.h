#ifndef __ASYNC_CALL_H__
#define __ASYNC_CALL_H__

#include <stddef.h>

enum {
    ASYNC_CALL_NOP,
    ASYNC_CALL_READ,
    ASYNC_CALL_WRITE,
};

struct request_ring_entry {
    uint8_t opcode;
    uint8_t flags;
    uint16_t _pad0;
    int32_t fd;
    uint64_t offset;
    uint64_t user_buf_addr;
    uint32_t buf_size;
    uint32_t _pad1;
    uint64_t user_data;
};

struct complete_ring_entry {
    uint64_t user_data;
    int32_t result;
    uint32_t _pad0;
};

struct request_ring {
    uint32_t* khead;
    uint32_t* ktail;
    uint32_t* capacity;
    struct request_ring_entry* entries;
};

struct complete_ring {
    uint32_t* khead;
    uint32_t* ktail;
    uint32_t* capacity;
    struct complete_ring_entry* entries;
};

struct async_call_buffer {
    struct request_ring req_ring;
    struct complete_ring comp_ring;
};

struct ring_offsets {
    uint32_t head;
    uint32_t tail;
    uint32_t capacity;
    uint32_t entries;
};

struct async_call_info {
    void* user_buf_ptr;
    size_t buf_size;
    struct ring_offsets req_off;
    struct ring_offsets comp_off;
};

int setup_async_call(int req_capacity, int comp_capacity, struct async_call_info* info,
                     size_t info_size);

int async_call_buffer_init(int req_capacity, int comp_capacity, struct async_call_buffer* buffer);

static inline void async_call_rw(int opcode, struct request_ring_entry* req, int fd,
                                 const void* addr, unsigned len, uint64_t offset)
{
    req->opcode = opcode;
    req->fd = fd;
    req->offset = offset;
    req->user_buf_addr = (uint64_t)addr;
    req->buf_size = len;
}

static inline void async_call_write(struct request_ring_entry* req, int fd, const void* addr,
                                    unsigned len, uint64_t offset)
{
    async_call_rw(ASYNC_CALL_WRITE, req, fd, addr, len, offset);
}

struct request_ring_entry* request_ring_get_entry(struct async_call_buffer* buffer, uint32_t idx);

struct complete_ring_entry* complete_ring_get_entry(struct async_call_buffer* buffer, uint32_t idx);

#endif // __ASYNC_CALL_H__
