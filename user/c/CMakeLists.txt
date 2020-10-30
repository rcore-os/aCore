cmake_minimum_required(VERSION 3.0)

project(acore_user)
enable_language(ASM)
set(CMAKE_OSX_DEPLOYMENT_TARGET "")

# Toolchain
set(PREFIX ${ARCH}-linux-musl-)

if (${ARCH} STREQUAL riscv32)
    set(CMAKE_C_FLAGS       "-march=rv32imac -mabi=ilp32 -mcmodel=medany")
elseif (${ARCH} STREQUAL riscv64)
    set(CMAKE_C_FLAGS       "-march=rv64imac -mabi=lp64 -mcmodel=medany")
else()
    message("Unsupported arch: ${ARCH}")
endif ()
set(CMAKE_ASM_COMPILER      ${PREFIX}gcc)
set(CMAKE_C_COMPILER        ${PREFIX}gcc)
set(CMAKE_RANLIB            ${PREFIX}ranlib)
set(CMAKE_C_FLAGS           "${CMAKE_C_FLAGS} -fno-builtin -nostdinc -fno-stack-protector -ggdb -Wall")
set(CMAKE_ASM_FLAGS         ${CMAKE_C_FLAGS})
set(CMAKE_C_LINK_FLAGS      "${LINK_FLAGS} -nostdlib")  # override default value to get rid of '-Wl,-search_paths_first -Wl,-headerpad_max_install_names'
set(CMAKE_SHARED_LIBRARY_LINK_C_FLAGS)                  # override default value to get rid of '-rdynamic' on Linux

# Path
if (${ARCH} STREQUAL riscv32 OR ${ARCH} STREQUAL riscv64)
    set(ARCH_DIR lib/arch/riscv)
else()
    set(ARCH_DIR lib/arch/${ARCH})
endif()

# Library
aux_source_directory(lib LIBS)
set(LIBS ${ARCH_DIR}/crt.S ${LIBS})
add_library(ulib ${LIBS} syscall_ids)
include_directories(include/)
target_include_directories(ulib PRIVATE ${ARCH_DIR})

# Execuatble
aux_source_directory(src SRCS)
set(EXECUTABLE_OUTPUT_PATH ${ARCH})
foreach(PATH ${SRCS})
    get_filename_component(NAME ${PATH} NAME_WE)
    add_executable(${NAME} ${PATH})
    target_link_libraries(${NAME} ulib)
endforeach()

add_custom_command(
    OUTPUT syscall_ids.h
    COMMAND sed ARGS -n -e s/__NR_/SYS_/p
        < ${CMAKE_SOURCE_DIR}/${ARCH_DIR}/syscall_ids.h.in
        > ${CMAKE_SOURCE_DIR}/lib/syscall_ids.h
)
