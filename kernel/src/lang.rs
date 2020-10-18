use core::alloc::Layout;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("\n{}", info);
    loop {}
}

#[lang = "oom"]
fn oom(_: Layout) -> ! {
    panic!("out of memory");
}
