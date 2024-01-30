//! This module presents the platform independent serial interface,
//! and handles backend selection.

// -------------------------------------------------------------------
//
// Public interface

/// Call a single time before any serial operations
pub fn setup() {device.serial_setup()}

/// Write a char out to serial. If not an ascii char, then this
/// should send multiple bytes.
pub fn put_char(c: char) {device.serial_put_char(c)}

/// This is a spin-blocking read from the primary serial port.
pub fn read_byte() -> u8 {device.serial_read_byte()}

/// This is a convience function for non-streaming prints. It is
/// preffered when possible.
pub fn put_string(s: &str) {device.serial_put_string(s)}

/// This is a convience wrapper for reading a known number of
/// bytes. It is prefered when possible.
pub fn read_bytes(buf: &mut [u8], num: u32) {device.serial_read_bytes(buf, num)}

// -------------------------------------------------------------------
//
// Backend selection

#[cfg(feature = "riscv-opensbi")]
mod opensbi;

#[cfg(feature = "riscv-opensbi")]
static device: opensbi::SerialProxy = opensbi::SERIAL_PROXY;

// -------------------------------------------------------------------
//
// Trait to talk to backends with

trait HALSerial {
    // start serial stuff. These should be used by most of the kernel
    // unless an extension / module takes control of the primary
    // serial port for specal managment / config later.
    //
    // Unless otherwise stated, these functions apply to the primary
    // serial port.
    //
    // TODO consider further buffering beyond hardware in the kernel's
    // view of the serial port. If so, add flush.

    /// Call a single time before any serial operations
    fn serial_setup(&self);

    /// Write a char out to serial. If not an ascii char, then this
    /// should send multiple bytes.
    fn serial_put_char(&self, c: char);

    /// This is a spin-blocking read from the primary serial port.
    fn serial_read_byte(&self) -> u8;

    /// This is a convience function for non-streaming prints. It is
    /// preffered when possible.
    fn serial_put_string(&self, s: &str);

    /// This is a convience wrapper for reading a known number of
    /// bytes. It is prefered when possible.
    fn serial_read_bytes(&self, buf: &mut [u8], num: u32);
}

