//! This module hides the platform details of a context switch.


// -------------------------------------------------------------------
//
// Public interface

/// This type should contain the initial info that needs to be
/// presereved through a context switch. This should not include
/// things like the kernel page table which from the point of view
/// of the main kernel should be switched automatically, but
/// should contian things like the current process. Think of it
/// like the arguments to the first function after a context
/// switch.
///
/// The implementation should provide a method of creating a
/// GPInfo structure, including a Process object to represent the
/// currently executing process.
#[cfg(feature = "riscv")]
pub type GPInfo = riscv::GPInfo;

// TODO ^ This is a slight break of form, but more elegant overall I
// think.

/// called once before any of the switching occurs, just like the
/// rest.
pub fn switch_setup() {device.switch_setup()}

/// Set the structure for the next restore on this CPU.
pub fn save_gp_info(gpi: GPInfo) {device.save_gp_info(gpi)}

/// Restore the most recently saved structure on this CPU.
pub fn restore_gp_info() -> GPInfo {device.restore_gp_info()}

// I think it makes sense for the unsafe / extern C boundary into
// the asm to be in the hal, so the main kernel just sees a safe
// never returning call, but I don't think the signature can be
// any more general than pc, pgtbl, sp. Porters can bring up
// issues if there are any later. Also you can't enforce the
// implementation of extern C functions in a trait, so what's the
// point?
//
// In spirit, here we need to make sure *somewhere* there is a
//
// extern "C" {pub fn process_resume_asm(pc: usize, pgtbl: usize, sp: usize) -> !;}
// extern "C" {pub fn process_start_asm(pc: usize, pgtbl: usize, sp: usize) -> !;}
// extern "C" {pub fn process_exit_rust(exit_code: isize) -> !;}
//
// implemented for every backing.

// -------------------------------------------------------------------
//
// Backend selection

#[cfg(feature = "riscv")]
mod riscv;

#[cfg(feature = "riscv")]
static device: riscv::ProxySwitch = riscv::SWITCHER;

// -------------------------------------------------------------------
//
// Trait to talk to backends with

pub trait HALSwitch {
    /// This type should contain the initial info that needs to be
    /// presereved through a context switch. This should not include
    /// things like the kernel page table which from the point of view
    /// of the main kernel should be switched automatically, but
    /// should contian things like the current process. Think of it
    /// like the arguments to the first function after a context
    /// switch.
    ///
    /// The implementation should provide a method of creating a
    /// GPInfo structure, including a Process object to represent the
    /// currently executing process.
    type GPInfo;

    /// called once before any of the switching occurs, just like the
    /// rest.
    fn switch_setup(&self);

    /// Set the structure for the next restore on this CPU.
    fn save_gp_info(&self, gpi: Self::GPInfo);

    /// Restore the most recently saved structure on this CPU.
    fn restore_gp_info(&self) -> Self::GPInfo;

    // I think it makes sense for the unsafe / extern C boundary into
    // the asm to be in the hal, so the main kernel just sees a safe
    // never returning call, but I don't think the signature can be
    // any more general than pc, pgtbl, sp. Porters can bring up
    // issues if there are any later. Also you can't enforce the
    // implementation of extern C functions in a trait, so what's the
    // point?
    //
    // In spirit, here we need to make sure *somewhere* there is a
    //
    // extern "C" {pub fn process_resume_asm(pc: usize, pgtbl: usize, sp: usize) -> !;}
    // extern "C" {pub fn process_start_asm(pc: usize, pgtbl: usize, sp: usize) -> !;}
    // extern "C" {pub fn process_exit_rust(exit_code: isize) -> !;}
    //
    // implemented for every backing.
}
