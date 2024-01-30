//! This module wraps platform specific details of interrupts and
//! exceptions. Naturally this means there isn't really a nice way to
//! have a unified interface. So we just provide a setup, and
//! platforms keep their complexity internally.

// -------------------------------------------------------------------
//
// Public interface

/// This function should set up and install all interupt and
/// exception handlers required by the system. It does not return
/// any errors, and instead should panic on error.
pub fn handler_setup() {backing.handler_setup()}

/// This function should be called after the above and by each cpu
/// that wants to be able to see interrupts.
pub fn local_setup() {backing.local_setup()}

// -------------------------------------------------------------------
//
// Backend selection

// TODO this is riscv-opensbi rather than just riscv to distinguish
// between machine mode operation and sup mode. Probably unecessary.
#[cfg(feature = "riscv-opensbi")]
mod opensbi;

#[cfg(feature = "riscv-opensbi")]
static backing: opensbi::ProxyHandler = opensbi::HANDLER;

// -------------------------------------------------------------------
//
// Traits to talk to backends

pub trait HALIntExc {
    fn handler_setup(&self);
    fn local_setup(&self);
}
