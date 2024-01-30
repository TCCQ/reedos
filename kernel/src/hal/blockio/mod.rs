//! This module does platform switching for the main blockio device
//! backend.

// -------------------------------------------------------------------
//
// Public interface
//
// TODO this interface is ripe for having uninitialized data flowing
// around. A rework seems in order. Not sure what is the most
// rustacious way to do this though.
//
// TODO slices?

/// Represents one block of data on disk. Data must point to 512 bytes
/// of owned memory.
#[repr(C)]
#[derive(Debug)]
pub struct Block {
    data: *mut u8,
    len: u32, // Multiple of 512 bytes.
    offset: u64,
}

/// This creation is device agnostic, since it is a handle for the
/// data once it is off the device and in memory. It's crude, but it
/// will have to do for now.
impl Block {
    // TODO: Hardcoded 4k block size. Prevent reading past fs block bounds.
    pub fn new(data: *mut u8, len: u32, offset: u64) -> Result<Self, ()> {
        if len % 512 == 0 && len <= 4096 {
            Ok(Self { data, len, offset })
        } else {
            Err(())
        }
    }
}

/// Must be called before any other io operations.
pub fn io_setup() {backing.io_setup()}

/// Sequence io operations.
pub fn io_barrier() {backing.io_barrier()}

/// Write a owned block out to disk
pub fn write_block(blk: &mut Block) {backing.write_block(blk)}

/// Read an owned block in from disk
pub fn read_block(blk: &mut Block) {backing.read_block(blk)}

/// Do whatever needs to be done on an interr upt
pub fn interrupt_respond() {backing.interrupt_respond()}

// -------------------------------------------------------------------
//
// Backend selection

#[cfg(feature = "virtio")]
mod virtio;

#[cfg(feature = "virtio")]
static backing: virtio::ProxyDev = virtio::DEVICE;

// -------------------------------------------------------------------
//
// Traits to talk to backends

/// Backings are expected to implement this for whatever a block looks
/// like to them. This module converts to and fro the `Block` type
/// defined here, and used everywhere else.
///
/// The reason we do this is to have a uniform struct facing out, but
/// avoid the issues of having the backings directly implement traits
/// for it. This would be an issue if there were more than one
/// backings trying to implement the same trait on the same outward
/// facing struct.
trait HALIO {
    // TODO I just matched these to the virtio ones, is this what we want?
    fn write_block(&self, blk: &mut Block);
    fn read_block(&self, blk: &mut Block);
    fn io_setup(&self);
    fn io_barrier(&self);
    fn interrupt_respond(&self);
}



