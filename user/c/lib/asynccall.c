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
    buffer->req_ring.capacity_mask = ptr + info.req_off.capacity_mask;
    buffer->req_ring.entries = ptr + info.req_off.entries;

    buffer->comp_ring.khead = ptr + info.comp_off.head;
    buffer->comp_ring.ktail = ptr + info.comp_off.tail;
    buffer->comp_ring.capacity = ptr + info.comp_off.capacity;
    buffer->comp_ring.capacity_mask = ptr + info.comp_off.capacity_mask;
    buffer->comp_ring.entries = ptr + info.comp_off.entries;
    return 0;
}

struct req_ring_entry* req_ring_get_entry(struct async_call_buffer* buffer, uint32_t idx)
{
    struct request_ring* req_ring = &buffer->req_ring;
    struct req_ring_entry* entry = &req_ring->entries[idx & *req_ring->capacity_mask];
    return entry;
}

struct comp_ring_entry* comp_ring_get_entry(struct async_call_buffer* buffer, uint32_t idx)
{
    struct completion_ring* comp_ring = &buffer->comp_ring;
    struct comp_ring_entry* entry = &comp_ring->entries[idx & *comp_ring->capacity_mask];
    return entry;
}
