//! Logging and printing macros

use core::fmt::{Write, Error};

use crate::hal::*;
use crate::lock::mutex::Mutex;
// use crate::sbi_nputs;

/// Wrapper for the HAL provided serial console. Ensure atomicity and nice rust bindings
pub static PRIMARY_SERIAL_PASS: Mutex<SerialPass> = Mutex::new(SerialPass {_ignore: ()});

pub struct SerialPass {
    _ignore: (),
}

impl Write for SerialPass {
    fn write_str(&mut self, out: &str) -> Result<(), Error> {
        HAL::serial_put_string(out);
        Ok(())
    }
}

macro_rules! print
{
    ($($args:tt)+) => ({
        use ::core::fmt::Write;
        // ^ we need this prefix :: to prevent conflits with other imported modules named core
        use crate::log;
        // LSP is confused by macros, this unsafe is required
        #[allow(unused_unsafe)]
        let mut dev = unsafe {log::PRIMARY_SERIAL_PASS.lock()};
        let _ = write!(dev, $($args)+);
    });
}

macro_rules! println
{
    () => ({
        print!("\r\n")
    });
    ($fmt:expr) => ({
        print!(concat!($fmt, "\r\n"))
    });
    ($fmt:expr, $($args:tt)+) => ({
        print!(concat!($fmt, "\r\n"), $($args)+)
    });
}

pub enum LogSeverity {
    Debug,
    Info,
    Warning,
    Error,
}

// use as `log::log!(Warning, "This is a test of the warning logging!");`
// in a while that has
// ```
// #[macro_use]
// pub mod log;
// ```
// at the top

macro_rules! log
{
    (Debug, $fmt:expr) => ({
        print!(concat!("[DEBUG] ", $fmt, "\r\n"))
    });
    (Info, $fmt:expr) => ({
        print!(concat!("[INFO] ", $fmt, "\r\n"))
    });
    (Warning, $fmt:expr) => ({
        print!(concat!("[WARN] ", $fmt, "\r\n"))
    });
    (Error, $fmt:expr) => ({
        print!(concat!("[ERROR] ", $fmt, "\r\n"))
    });

    (Debug, $fmt:expr, $($args:tt)+) => ({
        print!(concat!("[DEBUG] ", $fmt, "\r\n"), $($args)+)
    });
    (Info, $fmt:expr, $($args:tt)+) => ({
        print!(concat!("[INFO] ", $fmt, "\r\n"), $($args)+)
    });
    (Warning, $fmt:expr, $($args:tt)+) => ({
        print!(concat!("[WARN] ", $fmt, "\r\n"), $($args)+)
    });
    (Error, $fmt:expr, $($args:tt)+) => ({
        print!(concat!("[ERROR] ", $fmt, "\r\n"), $($args)+)
    });
}
