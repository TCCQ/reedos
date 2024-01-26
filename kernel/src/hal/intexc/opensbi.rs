//! This module contains the stuff we need for exception and interrupt
//! handling under opensbi.

use core::arch::{asm, global_asm};

#[cfg(all(feature = "riscv-opensbi", feature = "riscv-linkerscript"))]
global_asm!(include_str!("../asm/riscv/trap.s"));

use crate::hal::shared::riscv::plic;

pub type ProxyHandler = ();
pub static HANDLER: ProxyHandler = ();

impl super::HALIntExc for ProxyHandler {
    fn handler_setup(&self) {
        unsafe {
            asm!(
                // I should clear SIE here and restore later TODO
                "la {hold}, __strapvec",
                "csrw stvec, {hold}",
                hold = out(reg) _,
            );
        }
        plic::global_init();
    }

    fn local_setup(&self) {
        plic::local_init();
    }
}

fn read_scause() -> usize {
    unsafe {
        let out: usize;
        asm!(
            "csrr {out}, scause",
            out = out(reg) out
        );
        out
    }
}

fn read_stval() -> usize {
    unsafe {
        let out: usize;
        asm!(
            "csrr {out}, stval",
            out = out(reg) out
        );
        out
    }
}

/// These are the cause numbers for the regular s mode handler. I don't
/// see any reason they need to be public.
///
/// TODO how can we make these generic over 32/64 bit width?
const S_EXTERN_IRQ: usize = 0x9 | ( 1 << 63);
const S_STORE_AMO_FAULT: usize = 0xf;
const S_LOAD_PAGE_FAULT: usize = 0xd;

use crate::hal::layout::{stacks_start, stacks_end};
use crate::hal::vm::PAGE_SIZE;

/// Supervisor mode trap handler.
#[no_mangle]
pub extern "C" fn s_handler() {
    let cause = read_scause();

    match cause {
        S_EXTERN_IRQ => {
            s_extern()
        },
        S_STORE_AMO_FAULT => {
            // This is a write page fault (or a kind of write permission fault)

            let val = read_stval();

            // We want to catch stack over/underflow specifically;
            if val >= stacks_start() as usize &&
                val < (stacks_end() as usize + PAGE_SIZE) {
                    // error with the stack area. Hit a guard page

                    // TODO currently we can't tell if this is an over
                    // or an underflow, because we don't know which
                    // stack we were on originally. Our current HAL
                    // does not allow for known CPU ids. This could be
                    // changed, but I like the anonymity frankly
                    panic!("Stack over or underflow. Make sure you don't have a huge stack frame somewhere, or cut some recursion!");
                } else {
                    panic!("Store/AMO fault. Faulting address 0x{:x}", val);
                }
        },
        S_LOAD_PAGE_FAULT => {
            // This is a read page fault

            let val = read_stval();
            panic!("Load page fault. Faulting address 0x{:x}", val);
        },
        _ => {
            log!(
                Warning,
                "Uncaught supervisor mode interupt. scause: 0x{:x}",
                cause
            );
            panic!("s_handler panic")
        }
    }
}

/// Called when we get a S mode external interupt. Probably UART input
/// or virtio.
fn s_extern() {
    let irq = unsafe {
        plic::PLIC.get().expect("PLIC not initialized!").claim()
    };

    const UART_IRQ: u32 = plic::UART_IRQ as u32;
    const VIRTIO_IRQ: u32 = plic::VIRTIO_IRQ as u32;
    match irq {
        0 => {
            // reserved for "No interrupt" according to the
            // cookbook. Just chill I guess, I don't think we need to
            // complete it
        }
        UART_IRQ => {
            // I intentionally don't hold the lock here to
            // allow printing. Normally we shouldn't print
            // here


            // TODO figure out how UART input works with opensbi. It's
            // just whatever the opensbi getChar is right? Is there a
            // non-blocking version, or an interrupting version? does
            // it have buffering?

            panic!("Unexpected UART input interrupt. Are you not using opensbi?");
        },
        VIRTIO_IRQ => {
            // TODO I am assuming blindly that virtio works as normal
            // under opensbi. We can dump PLIC registers on virt to
            // figure that out I guess.
            crate::hal::blockio::interrupt_respond();
            unsafe {
                plic::PLIC.get().unwrap().complete(irq)
            };
        },
        _ => {
            panic!("Uncaught PLIC exception.")
        }
    }
}

extern "C" {
    pub fn __mtrapvec();
    pub fn __strapvec();
}
