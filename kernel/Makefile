ARCH ?= riscv64
MODE ?= debug
SMP ?= 2

OBJDUMP := rust-objdump
OBJCOPY := rust-objcopy

ifeq ($(ARCH), riscv64)
target := riscv64imac-unknown-none-elf
else
$(error Unsupported architecture $(ARCH))
endif

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
	-device loader,file=$(kernel_img),addr=0x80200000
endif

.PHONY: build kernel run qemu asm clean

build: $(kernel_img)

kernel:
	@echo Building aCore kernel
	cargo build $(build_args)

$(kernel_img): kernel
	$(OBJCOPY) $(kernel) --strip-all -O binary $@

run: build qemu

qemu:
	$(qemu) $(qemu_args)

asm:
	$(OBJDUMP) -d $(kernel) | less

clean:
	cargo clean
