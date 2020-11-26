ARCH ?= riscv64
MODE ?= debug
SMP ?= 2
LOG ?=
USER_ELF ?= ../user/build/$(ARCH)/read_after_write

export ARCH

sysroot := $(shell rustc --print sysroot)
OBJDUMP ?= $(shell find $(sysroot) -name llvm-objdump)
OBJCOPY ?= $(shell find $(sysroot) -name llvm-objcopy)

target := $(shell cat targets.txt | grep $(ARCH))
qemu := qemu-system-$(ARCH)
build_path := target/$(target)/$(MODE)
kernel := $(build_path)/acore
kernel_img := $(build_path)/acore.img

build_args := --target $(target)
ifeq ($(MODE), release)
build_args += --release
endif

qemu_args := -nographic -smp $(SMP)
ifeq ($(ARCH), riscv64)
qemu_args += \
	-machine virt \
	-bios default \
	-m 1G \
	-device loader,addr=0x80200000,file=$(kernel_img) \
	-device loader,addr=0x90000000,file=$(USER_ELF),force-raw=on
endif

.PHONY: build kernel run qemu debug asm clean

build: $(kernel_img)

clippy:
	cargo clippy $(build_args) -- -D warnings

kernel:
	@echo Building aCore kernel
	cargo build $(build_args)

$(kernel_img): kernel
	$(OBJCOPY) $(kernel) --strip-all -O binary $@

run: build qemu

qemu:
	$(qemu) $(qemu_args)

debug:
	$(qemu) $(qemu_args) -s -S &
	@sleep 1
	gdb $(kernel) -ex "target remote :1234"

asm:
	$(OBJDUMP) -d $(kernel) | less

clean:
	cargo clean
