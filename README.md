# aCore

Asynchronous OS kernel written in Rust.

🚧 Working In Progress

## Getting started

### Setup

```bash
$ rustup component add rust-src llvm-tools-preview
$ rustup target add riscv64imac-unknown-none-elf
```

### Build & Run

```bash
$ cd kernel
$ make run [ARCH=riscv64] [MODE=release]
```
