//! This mod should include all the platform specific stuff we need to
//! discover or create the kernel memory map. This is not the same as
//! the kernel page table, but the information here is used to make
//! that.

// -------------------------------------------------------------------
//
// Public interface

// TODO Do we need this? might for active device discovery
pub fn sections_setup() {backing.sections_setup()}

pub fn text_start() -> *mut usize {backing.text_start()}
pub fn text_end() -> *mut usize {backing.text_end()}

pub fn rodata_start() -> *mut usize {backing.rodata_start()}
pub fn rodata_end() -> *mut usize {backing.rodata_end()}

pub fn data_start() -> *mut usize {backing.data_start()}
pub fn data_end() -> *mut usize {backing.data_end()}

pub fn stacks_start() -> *mut usize {backing.stacks_start()}
pub fn stacks_end() -> *mut usize {backing.stacks_end()}

pub fn intstacks_start() -> *mut usize {backing.intstacks_start()}
pub fn intstacks_end() -> *mut usize {backing.intstacks_end()}

pub fn bss_start() -> *mut usize {backing.bss_start()}
pub fn bss_end() -> *mut usize {backing.bss_end()}

pub fn memory_start() -> *mut usize {backing.memory_start()}
pub fn memory_end() -> *mut usize {backing.memory_end()}


// -------------------------------------------------------------------
//
// Backend selection

#[cfg(feature = "riscv-linkerscript")]
mod riscvbaked;

#[cfg(feature = "riscv-linkerscript")]
static backing: riscvbaked::ProxyLoc = riscvbaked::LOCATIONS;


// -------------------------------------------------------------------
//
// Traits to talk to backends

// This wraps all hardware discovery.
//
// TODO when we do device tree stuff, it will be exposed here
// trait HALDiscover {
//     fn discover_setup();

//     // Const in the short term, but we will see
//     const NHART: usize;

//     // I think this is safe to be a const. It can change when we have
//     // reason for it.
//     const DRAM_BASE: *mut usize;
// }

/// Provide info about the location of sections of the kernel
/// binary. This trait must be be safe to call before allocation is
/// brought up.
trait HALSections {
    /// This may very well be empty
    fn sections_setup(&self);

    fn text_start(&self) -> *mut usize;
    fn text_end(&self) -> *mut usize;

    fn rodata_start(&self) -> *mut usize;
    fn rodata_end(&self) -> *mut usize;

    fn data_start(&self) -> *mut usize;
    fn data_end(&self) -> *mut usize;

    fn stacks_start(&self) -> *mut usize;
    fn stacks_end(&self) -> *mut usize;

    fn intstacks_start(&self) -> *mut usize;
    fn intstacks_end(&self) -> *mut usize;

    fn bss_start(&self) -> *mut usize;
    fn bss_end(&self) -> *mut usize;

    // TODO deprecate? wait for clarity as for context switches under
    // opensbi
    fn memory_start(&self) -> *mut usize;
    fn memory_end(&self) -> *mut usize;
}
