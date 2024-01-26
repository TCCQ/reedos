//! This module contains the opensbi interface for CPU control.
//!
//! TODO unstub this

use crate::hal::cpucontrol::HALCPUError;

pub type ProxyController = ();
pub static CPU_CONTROLLER: ProxyController = ();

impl super::HALCPU for ProxyController {
    fn isolate(&self) {
        // intentionally empty, opensbi only starts one cpu on
        // coldboot, and that's all we care about right now.
    }

    fn wake_one<F: Fn() -> !>(&self, _start: F) -> Result<(), HALCPUError> {
        todo!("opensbi call for waking harts")
        // TODO it would be good to have a central place for the rust
        // wrappers for things like opensbi calls. Could be like a
        // `platform utils` module or somthing. Controlled by flags of
        // course.
    }
}


