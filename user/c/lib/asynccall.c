#include <asynccall.h>
#include <unistd.h>

int async_call_buffer_init(int req_capacity, int comp_capacity, struct async_call_buffer* buffer)
{
    struct async_call_info info;
    int ret = setup_async_call(req_capacity, comp_capacity, &info, sizeof(info));
    if (ret < 0) {
        return ret;
    }
    void* ptr = info.user_buf_ptr;
    buffer->req_ring.khead = ptr + info.req_off.head;
    buffer->req_ring.ktail = ptr + info.req_off.tail;
    buffer->req_ring.capacity = ptr + info.req_off.capacity;
    buffer->req_ring.entries = ptr + info.req_off.entries;

    buffer->comp_ring.khead = ptr + info.comp_off.head;
    buffer->comp_ring.ktail = ptr + info.comp_off.tail;
    buffer->comp_ring.capacity = ptr + info.comp_off.capacity;
    buffer->comp_ring.entries = ptr + info.comp_off.entries;
    return 0;
}

struct request_ring_entry* request_ring_get_entry(struct async_call_buffer* buffer, uint32_t idx)
{
    struct request_ring* req_ring = &buffer->req_ring;
    struct request_ring_entry* req = &req_ring->entries[idx];
    return req;
}

struct complete_ring_entry* complete_ring_get_entry(struct async_call_buffer* buffer, uint32_t idx)
{
    struct complete_ring* comp_ring = &buffer->comp_ring;
    struct complete_ring_entry* comp = &comp_ring->entries[idx];
    return comp;
}
