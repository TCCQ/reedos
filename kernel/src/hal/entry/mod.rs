//! This mod does the switching on flags to include the proper stuff
//! for kernel entry.

use core::arch::global_asm;

#[cfg(all(feature = "riscv-opensbi", feature = "riscv-linkerscript"))]
global_asm!(include_str!("../asm/smodestart.s"));


