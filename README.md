# aCore

[![Actions Status](https://github.com/rcore-os/aCore/workflows/CI/badge.svg)](https://github.com/rcore-os/aCore/actions)

Asynchronous OS kernel written in Rust.

🚧 Working In Progress

## Getting Started

### Setup Environment

```bash
$ rustup component add rust-src llvm-tools-preview
$ rustup target add riscv64imac-unknown-none-elf
```

### Build & Run

```bash
$ cd kernel
$ make run [ARCH=riscv64] [LOG=info] [MODE=release]
```
