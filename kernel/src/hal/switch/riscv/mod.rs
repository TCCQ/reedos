//! Riscv implementation of a context switch. Relies on setup that is
//! done in other parts of the HAL tree. Specifically some stack setup
//! on kernel entry.

use core::arch::{asm, global_asm};

#[cfg(all(feature = "riscv-opensbi", feature = "riscv-linkerscript"))]
global_asm!(include_str!("../../asm/riscv/trampoline.s"));

mod hartlocal;

pub type GPInfo = hartlocal::GPInfo;

pub type ProxySwitch = ();
pub static SWITCHER: ProxySwitch = ();

impl super::HALSwitch for ProxySwitch {
    type GPInfo = hartlocal::GPInfo;

    fn switch_setup(&self) {
        hartlocal::hartlocal_info_interrupt_stack_init();
    }

    fn save_gp_info(&self, gpi: Self::GPInfo) {
        hartlocal::save_gp_info64(gpi);
    }

    fn restore_gp_info(&self) -> Self::GPInfo {
        hartlocal::restore_gp_info64()
    }
}

#[no_mangle]
pub extern "C" fn scall_rust(a0: usize, a1: usize, a2: usize, a3: usize,
                             a4: usize, a5: usize, a6: usize, a7: usize)
{
    let proc_pc: usize;
    let proc_sp: usize;
    unsafe {
        asm!(
            "mv {pc}, s2",
            "mv {sp}, s3",
            pc = out(reg) proc_pc,
            sp = out(reg) proc_sp
        );
    }
    crate::process::scall_rust_standard(a0,a1,a2,a3,a4,a5,a6,a7, proc_pc, proc_sp)
}

