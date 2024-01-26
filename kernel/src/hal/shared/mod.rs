//! This module doesn't have a public interface, but should be the
//! root to include platform specific modules that don't fit nicely
//! into the tree anywhere else.


#[cfg(feature = "riscv")]
pub mod riscv;

#[cfg(feature = "riscv-opensbi")]
pub mod opensbi;
