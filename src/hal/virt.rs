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

use core::arch::asm;

use super::*;

impl HALSerial for HAL {
    // TODO use the debug version extension instead
    fn serial_setup() {
        // empty, opensbi does it for us
        //
        // TODO do extension checking to make sure opensbi has the
        // extension for consoles
    }

    fn serial_put_char(c: char) {
        let mut buffer: [u8; 4];
        let slice = c.encode_utf8(&buffer);
        for iter in slice.as_bytes() {
            let val: u8 = iter.clone();
            asm!(
                "mv a0, {value}",
                "li a6, 0",
                "li a7, 1",
                "scall",
                value = in(reg) val,
            );
            // opensbi console putchar
        }
    }

    fn serial_read_byte() -> u8 {
        let val: u8;
        asm!(
            "mv {value}, a0",
            "li a6, 0",
            "li a7, 2",
            "scall",
            value = out(reg) val,
        );
        // opensbi console getchar
        val
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

}

impl HALBacking for HAL {

}
