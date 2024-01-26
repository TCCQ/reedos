//! This module contains riscv opensbi interations. Notably serial
//! output and input. This is not the only feature flag you need to
//! use opensbi, see hal.rs.

use crate::hal::shared::opensbi::*;

/// Implementer of the logging capabilities of opensbi
pub type SerialProxy = ();
pub static SERIAL_PROXY: SerialProxy = ();

impl super::HALSerial for SerialProxy {
    fn serial_setup(&self) {
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

    fn serial_put_char(&self, c: char) {
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

    fn serial_read_byte(&self) -> u8 {
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

    fn serial_put_string(&self, s: &str) {
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
    fn serial_read_bytes(&self, buf: &mut [u8], num: u32) {
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

