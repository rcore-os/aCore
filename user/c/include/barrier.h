#ifndef __BARRIER_H__
#define __BARRIER_H__

#if defined(__x86_64) || defined(__i386__)
#define read_barrier()  __asm__ __volatile__("" ::: "memory")
#define write_barrier() __asm__ __volatile__("" ::: "memory")
#elif defined(__riscv_xlen)
#define read_barrier()  __asm__ __volatile__("fence r,rw" ::: "memory")
#define write_barrier() __asm__ __volatile__("fence rw,w" ::: "memory")
#else
/*
 * Add arch appropriate definitions. Be safe and use full barriers for
 * archs we don't have support for.
 */
#define read_barrier()  __sync_synchronize()
#define write_barrier() __sync_synchronize()
#endif

#define smp_store_release(p, v)                                                                    \
    do {                                                                                           \
        write_barrier();                                                                           \
        *p = v;                                                                                    \
    } while (0)

#define smp_load_acquire(p)                                                                        \
    ({                                                                                             \
        typeof(*p) ret = *p;                                                                       \
        read_barrier();                                                                            \
        ret;                                                                                       \
    })

#endif // __BARRIER_H__
