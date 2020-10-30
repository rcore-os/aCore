ARCH ?= riscv64
MODE ?= release

out_dir := build/$(ARCH)

cmake_build_args := -DARCH=$(ARCH)
ifeq ($(MODE), release)
cmake_build_args += -DCMAKE_BUILD_TYPE=Release
else ifeq ($(MODE), debug)
cmake_build_args += -DCMAKE_BUILD_TYPE=Debug
endif

all: c

c:
	@echo Building aCore user app
	@mkdir -p c/build
	@cd c/build && cmake $(cmake_build_args) .. && make -j
	@mkdir -p $(out_dir)
	@cp c/build/$(ARCH)/* $(out_dir)

clean:
	@rm -rf c/build build

.PHONY: build clean c
