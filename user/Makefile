ARCH ?= riscv64
PREFIX ?= $(ARCH)-linux-musl-
TARGET ?= test

SRC := $(TARGET).c

CC := $(PREFIX)gcc
OBJDUMP := $(PREFIX)objdump
CFLAGS := -nostdlib

all: $(TARGET)

$(TARGET): $(SRC)
	$(CC) $< -o $@ $(CFLAGS)

asm: $(TARGET)
	$(OBJDUMP) -d $(TARGET) | less

clean:
	rm test
