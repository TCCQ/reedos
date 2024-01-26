//! This module hides platform specific details of CPU control. It's
//! mostly for isolating single threads to do init with, and then
//! undoing said action.

// -------------------------------------------------------------------
//
// Public interface

/// Call by all cpus, a single one will return. Others can be
/// retrieved later with wake_one. This call should be used before
/// any setup, and is only valid to call a single time. If a
/// platform or hardware guarantees that only one CPU will wake on
/// a cold boot, this call can be empty, however wake_one should
/// still function as expected. (This is the case for opensbi).
///
/// If this call cannot succeed, it should panic rather than
/// return an error.
pub fn isolate() {backing.isolate()}

/// Retrieve some CPU from the isolate call. This call is only
/// valid after the single textural call to isolate. It will
/// return an error or nothing to the caller depending on if the
/// wakeup was successful. A platform should have some other way
/// of determing the number of CPUs. If a CPU is woken, it's
/// execution starts at the passed function.
///
/// This should probably only be called after global_setup
pub fn wake_one<F: Fn() -> !>(start: F) -> Result<(), HALCPUError> {
    backing.wake_one(start)
}

// -------------------------------------------------------------------
//
// Backend selection

#[cfg(feature = "riscv-opensbi")]
mod opensbi;

#[cfg(feature = "riscv-opensbi")]
static backing: opensbi::ProxyController = opensbi::CPU_CONTROLLER;

// -------------------------------------------------------------------
//
// Traits to talk to backends

pub enum HALCPUError {
    OutOfCPU,
}

trait HALCPU {
    fn isolate(&self);

    fn wake_one<F: Fn() -> !>(&self, start: F) -> Result<(), HALCPUError>;
}

