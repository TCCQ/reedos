/// This module is conditionally included and contains the HAL backing
/// for the qemu riscv 'virt' machine. It assumes that the kernel is
/// booted in S mode by uboot and is running on top of opensbi.

// Useful:
//
// https://github.com/riscv-non-isa/riscv-sbi-doc/blob/master/riscv-sbi.adoc

// TODO
//
// add failstate logging, for use with panics.
//
// add return value checking for opensbi calls

use core::arch::{asm, global_asm};

use super::*;

const DEBUG_EID: u32 = 0x4442434E;
const BASE_EID: u32 = 0x10;

const SBI_SUCCESS: i32               =  0; // Completed successfully
const SBI_ERR_FAILED: i32            = -1; // Failed
const SBI_ERR_NOT_SUPPORTED: i32     = -2; // Not supported
const SBI_ERR_INVALID_PARAM: i32     = -3; // Invalid parameter(s)
const SBI_ERR_DENIED: i32            = -4; // Denied or not allowed
const SBI_ERR_INVALID_ADDRESS: i32   = -5; // Invalid address(s)
const SBI_ERR_ALREADY_AVAILABLE: i32 = -6; // Already available
const SBI_ERR_ALREADY_STARTED: i32   = -7; // Already started
const SBI_ERR_ALREADY_STOPPED: i32   = -8; // Already stopped
const SBI_ERR_NO_SHMEM: i32          = -9; // Shared memory not available

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


fn opensbi_call(eid: u32, fid: u32, a0: u32, a1: u32, a2: u32, a3: u32) -> (i32, u32) {
    _opensbi_call(eid as usize, fid as usize, a0 as usize, a1 as usize, a2 as usize, a3 as usize)
}

global_asm!(
    "_opensbi_call:",
    "mv a7, a0",
    "mv a6, a1",
    "mv a0, a2",
    "mv a1, a3",
    "mv a2, a4",
    "mv a3, a5",
    "ecall",
    "ret",
);

// fn opensbi_call(eid: u32, fid: u32, mut a0: u32, mut a1: u32, a2: u32, a3: u32) -> (i32, u32) {
//     unsafe {
//         asm!(
//             "ecall",
//             inout("a0") a0,
//             inout("a1") a1,
//             in("a2") a2,
//             in("a3") a3,
//             in("a6") fid,
//             in("a7") eid,
//         );
//     }
//     (a0 as i32, a1)                    // err, val
// }

impl HALSerial for HAL {
    fn serial_setup() {
        // probe for opensbi debug console extension
        let (err, val) = opensbi_call(BASE_EID, 3, DEBUG_EID, 0, 0, 0);
        match err {
            0 => {
                // all good!
                match val {
                    0 => {
                        panic!("Opensbi does not support debug logging!");
                    },
                    1 => {
                        // exactly as we want
                    },
                    _ => {
                        panic!("Unexpected opensbi return code!");
                    }
                }
            },
            SBI_ERR_FAILED | SBI_ERR_NOT_SUPPORTED | SBI_ERR_DENIED => {
                panic!("Unsupported base opensbi extension!");
            },
            _ => {
                panic!("Unexpected opensbi error code!");
            }
        }
    }

    fn serial_put_char(c: char) {
        let mut buffer: [u8; 4] = [0; 4];
        let slice = c.encode_utf8(&mut buffer);
        for iter in slice.as_bytes() {
            let val: u8 = iter.clone();
            // opensbi console putchar
            let (err, _) = opensbi_call(DEBUG_EID, 2, val as u32, 0, 0, 0);
            match err {
                0 => {},
                SBI_ERR_FAILED => {
                    panic!("Failed to write to console!");
                },
                _ => {
                    panic!("Unexpected opensbi error code!");
                }
            }
        }
    }

    fn serial_read_byte() -> u8 {
        let mut val: u8 = 0;
        // opensbi console getchar
        let (err, _ret) = opensbi_call(DEBUG_EID, 1,
                     1,         // 1 byte
                     ((&mut val as *mut u8 as usize) & 0xFF_FF_FF_FF) as u32, // low bits
                     ((&mut val as *mut u8 as usize) >> 32) as u32,                  // high bits
                     0,
        );
        match err {
            SBI_SUCCESS => {},
            SBI_ERR_FAILED => {
                panic!("Opensbi I/O fail on read");
            }
            SBI_ERR_INVALID_PARAM => {
                panic!("Opensbi didn't like arguments to console read");
            },
            _ => {
                panic!("Unexpected opensbi error code");
            }
        }
        val
    }

    fn serial_put_string(s: &str) {
        let (err, _ret) = opensbi_call(DEBUG_EID, 0,
                     s.len() as u32,         // 1 byte
                     ((s.as_ptr() as usize) & 0xFF_FF_FF_FF) as u32, // low bits
                     ((s.as_ptr() as usize) >> 32) as u32,                  // high bits
                     0,
        );
        match err {
            SBI_SUCCESS => {},
            SBI_ERR_FAILED => {
                panic!("Opensbi I/O fail on write");
            }
            SBI_ERR_INVALID_PARAM => {
                panic!("Opensbi didn't like arguments to console write");
            },
            _ => {
                panic!("Unexpected opensbi error code");
            }
        }

    }

    // TODO pass errors back up
    fn serial_read_bytes(buf: &mut [u8], num: u32) {
        let (err, _ret) = opensbi_call(DEBUG_EID, 1,
                     num,         // 1 byte
                     ((buf.as_mut_ptr() as usize) & 0xFF_FF_FF_FF) as u32, // low bits
                     ((buf.as_mut_ptr() as usize) >> 32) as u32,                  // high bits
                     0,
        );
        match err {
            SBI_SUCCESS => {},
            SBI_ERR_FAILED => {
                panic!("Opensbi I/O fail on read");
            }
            SBI_ERR_INVALID_PARAM => {
                panic!("Opensbi didn't like arguments to console read");
            },
            _ => {
                panic!("Unexpected opensbi error code");
            }
        }

    }
}

impl HALTimer for HAL {
    // TODO we need mtime for mtimecmp type stuff here. See the comment in hal.rs
    fn timer_setup() {
        todo!()
    }

    fn timer_set(ticks: u64) {
        todo!()
    }
}

impl HALVM for HAL {
    fn pgtbl_setup() {
        todo!()
    }

    fn pgtbl_new_empty() -> Result<PageTable, HALVMError> {
        todo!()
    }

    fn pgtbl_deep_copy(src: PageTable, dest: PageTable) -> Result<(), HALVMError> {
        todo!()
    }

    fn pgtbl_insert_leaf(pgtbl: PageTable, phys: Address, virt: Address, flags: PageMapFlags) -> Result<(), HALVMError> {
        todo!()
    }

    fn pgtbl_remove(pgtbl: PageTable, virt: Address) -> Result<(), HALVMError> {
        todo!()
    }
}

impl HALBacking for HAL {
    fn global_setup() {
        assert!(opensbi_call(BASE_EID, 0, 0, 0, 0, 0).1 == (1<<24) | (0 & 0xFF_FF_FF), "Wrong sbi version");
        Self::serial_setup();
        // Self::timer_setup();
        // Self::pgtbl_setup();
    }
}
