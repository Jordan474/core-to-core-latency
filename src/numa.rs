use std::mem::size_of;
use std::arch::asm;
use std::ptr::{null_mut, addr_of};

use ::libc;

// /usr/include/linux/mempolicy.h
const MPOL_MF_STRICT: u32 = 1 << 0;
const MPOL_MF_MOVE: u32 = 1 << 1;


pub fn move_to_numa_node<T>(x: *const T) {
    let page_size: usize = unsafe { libc::sysconf(libc::_SC_PAGE_SIZE) }.try_into().unwrap();
    assert!(page_size > 0 && page_size.is_power_of_two());

    let numanode = 0usize;
    let res = unsafe {
        let ptr = x.cast::<libc::c_void>().cast_mut();
        // let offset = *ptr.cast::<usize>() & (page_size - 1);
        // println!("mbind {ptr:?} offset {offset:x}");
        mbind(
            // ptr.sub(offset),
            ptr,
            size_of::<T>(),
            libc::MPOL_LOCAL,
            null_mut(),
            0,
            // &numanode,
            // 1,
            MPOL_MF_STRICT | MPOL_MF_MOVE,
        )
    };
    if res != 0 {
        let errstr = (-res)
            .try_into()
            .map(|e| strerror_string(e))
            .unwrap_or_else(|e| format!("Unknown error code {e}"));
        panic!("mbind failed ({res} {errstr})");
    }
}

fn strerror_string(errorcode: i32) -> String {
    let charstar = unsafe {
        let mut buf = [0u8; 2048];
        libc::strerror_r(errorcode, buf.as_mut_ptr().cast::<i8>(), buf.len() - 1);
        buf
    };
    String::from_utf8_lossy(&charstar[..]).to_string()
}

// https://github.com/bastion-rs/allocator-suite/blob/master/src/memory_sources/mmap/numa/numa_settings.rs
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline(always)]
unsafe fn mbind(
    start: *mut libc::c_void,
    len: usize,
    mode: i32,
    nodemask: *const usize,
    maxnode: usize,
    flags: u32,
) -> isize {
    let result: isize;
    asm!(
        "syscall",
        in("rax") libc::SYS_mbind, // syscall number
        in("rdi") start,
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
