#![cfg(unix)]

use std::ptr::{NonNull, null_mut};
use std::mem::size_of;
use std::arch::asm;

// // /usr/include/linux/mempolicy.h
// const MPOL_DEFAULT: i32 = 0;
// const MPOL_PREFERRED: i32 = 1;
// const MPOL_BIND: i32 = 2;
// const MPOL_INTERLEAVE: i32 = 3;
// const MPOL_LOCAL: i32 = 4;
// const MPOL_PREFERRED_MANY: i32 = 5;

// // /usr/include/linux/mman.h
// const MAP_SHARED: i32 = 0x01;
// const MAP_PRIVATE: i32 = 0x02;
// const MAP_SHARED_VALIDATE: i32 = 0x03;

use ::libc::*;

/// Move T to a memory allocation bound to local numa node, if available.
pub fn allocate_membind_here<T>(x: T) -> NonNull<T> {
    let size = size_of::<T>();
    let ptr = mmap_membind_local(size).cast::<T>();
    // TODO: check alignment
    unsafe { ptr.as_ptr().write(x); };
    ptr
}

/// Allocate memory ready bound to the current node, if available.
fn mmap_membind_local(size: usize) -> NonNull<c_void> {
    let ptr = unsafe {
        mmap(
            null_mut(),
            size,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE,
            -1i32,
            0
        )
    };
    if ptr == MAP_FAILED {
        panic!("mmap failed");
    }

    const MPOL_MF_STRICT: u32 = 1 << 0;
    // const MPOL_MF_MOVE: u32 = 1 << 1;

    let res = unsafe {
        mbind(
            ptr,
            size,
            MPOL_LOCAL,
            null_mut(),
            0,
            MPOL_MF_STRICT,
        )
    };
    if res != 0 {
        panic!("mbind failed ({res})");
    }

    NonNull::new(ptr).unwrap()
}

// https://github.com/bastion-rs/allocator-suite/blob/master/src/memory_sources/mmap/numa/numa_settings.rs
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline(always)]
unsafe fn mbind(
    start: *mut c_void,
    len: usize,
    mode: i32,
    nodemask: *const usize,
    maxnode: usize,
    flags: u32,
) -> isize {
    let result: isize;
    asm!(
        "syscall",
        in("rax") SYS_mbind, // syscall number
        in("rdi") start, // fd (stdout)
        in("rsi") len,
        in("rdx") mode,
        in("r10") nodemask,
        in("r8") maxnode,
        in("r9") flags,
        out("rcx") _, // clobbered by syscalls
        out("r11") _, // clobbered by syscalls
        lateout("rax") result,
    );
    result
}
