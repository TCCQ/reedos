//! This module contains/includes the asm nessesary for the virt
//! backing. All asm is necessarily hardware specific, thus it must be
//! here. There is likely high asm reuse between backings that share
//! an ISA.

use core::arch::global_asm;

// for riscv w/ opensbi + uboot specifically, we need to include
// _entry, our initial smode entry, where we setup stacks. See the
// linkerscript for details.

global_asm!(include_str!("asm/macro.s"));
global_asm!(include_str!("asm/smodestart.s"));
global_asm!(include_str!("asm/trap.s"));
global_asm!(include_str!("asm/trampoline.s"));
