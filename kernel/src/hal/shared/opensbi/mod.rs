//! Shared opensbi stuff

use core::arch::asm;

pub const DEBUG_EID: u32 = 0x4442434E;
pub const BASE_EID: u32 = 0x10;

pub const SBI_SUCCESS: i32               =  0; // Completed successfully
pub const SBI_ERR_FAILED: i32            = -1; // Failed
pub const SBI_ERR_NOT_SUPPORTED: i32     = -2; // Not supported
pub const SBI_ERR_INVALID_PARAM: i32     = -3; // Invalid parameter(s)
pub const SBI_ERR_DENIED: i32            = -4; // Denied or not allowed
pub const SBI_ERR_INVALID_ADDRESS: i32   = -5; // Invalid address(s)
pub const SBI_ERR_ALREADY_AVAILABLE: i32 = -6; // Already available
pub const SBI_ERR_ALREADY_STARTED: i32   = -7; // Already started
pub const SBI_ERR_ALREADY_STOPPED: i32   = -8; // Already stopped
pub const SBI_ERR_NO_SHMEM: i32          = -9; // Shared memory not available

pub fn opensbi_call(eid: u32, fid: u32, a0: u32, a1: u32, a2: u32, a3: u32) -> (i32, u32) {
    _opensbi_call(eid as usize, fid as usize, a0 as usize, a1 as usize, a2 as usize, a3 as usize)
}

fn _opensbi_call(eid: usize, fid: usize, mut a0: usize, mut a1: usize, a2: usize, a3: usize) -> (i32, u32) {
    unsafe {
        asm!(
            "ecall",
            in("a7") eid,
            in("a6") fid,
            inout("a0") a0,
            inout("a1") a1,
            in("a2") a2,
            in("a3") a3,
        );
    }
    (a0 as i32, a1 as u32)
}

