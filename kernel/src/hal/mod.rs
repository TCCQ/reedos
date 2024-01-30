//! This module tree should contain the details of the hardware
//! abstraction layer.


mod shared;
pub mod serial;
pub mod layout;
pub mod discover;
pub mod blockio;
pub mod cpucontrol;
pub mod intexc;
pub mod vm;
pub mod switch;

// pub trait HALTimer {
//     /// Call once before any timer use
//     fn timer_setup();

//     /// Set a timer to go off a single time.
//     ///
//     /// TODO how to set a meaning of a tick that is reasonable across
//     /// hardwares. RISC-V uses mtime, which is not even fully defined
//     /// there. See priv spec.
//     ///
//     /// The natural thing is to do realtime, but I'm not sure how to
//     /// convert mtime to realtime
//     fn timer_set(ticks: u64);

//     // TODO timer clear? timers are one time only, so ideally don't
//     // start ones that you don't wnat to happen
// }



