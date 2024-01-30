//! This module is to cover platform specific hardware discovery. It
//! is currently incomplete.
//!
//! TODO do the opensbi stuff here

// -------------------------------------------------------------------
//
// Public interface


// TODO see opensbi submodule
#[cfg(feature = "riscv-opensbi")]
pub const NHART: usize = 2;
#[cfg(feature = "riscv-opensbi")]
pub const DRAM_BASE: *mut usize = 0x80000000 as *mut usize;

pub fn discover_setup() {backing.discover_setup()}

// -------------------------------------------------------------------
//
// Backend selection

#[cfg(feature = "riscv-opensbi")]
mod opensbi;

#[cfg(feature = "riscv-opensbi")]
static backing: opensbi::DiscoveryProxy = opensbi::DISCOVERER;

// -------------------------------------------------------------------
//
// Trait to talk to backends with

trait HALDiscover {
    fn discover_setup(&self);
}
