/// This module should contain the details of the hardware abstraction
/// layer

use bitflags::bitflags;

#[cfg(feature = "hal-virt")]
pub mod virt;

struct HAL {}

// TODO add a HAL trait for interupts

pub trait HALSerial {
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
    fn serial_setup();

    /// Write a char out to serial. If not an ascii char, then this
    /// should send multiple bytes.
    fn serial_put_char(c: char);

    /// This is a spin-blocking read from the primary serial port.
    fn serial_read_byte() -> u8;

    /// This is a convience function for non-streaming prints. It is
    /// preffered when possible.
    fn serial_put_string(s: &str);

    /// This is a convience wrapper for reading a known number of
    /// bytes. It is prefered when possible.
    fn serial_read_bytes(buf: &mut [u8], num: u32);
}

pub trait HALTimer {
    /// Call once before any timer use
    fn timer_setup();

    /// Set a timer to go off a single time.
    ///
    /// TODO how to set a meaning of a tick that is reasonable across
    /// hardwares. RISC-V uses mtime, which is not even fully defined
    /// there. See priv spec.
    ///
    /// The natural thing is to do realtime, but I'm not sure how to
    /// convert mtime to realtime
    fn timer_set(ticks: u64);

    // TODO timer clear? timers are one time only, so ideally don't
    // start ones that you don't wnat to happen
}


// These are outside to force them to be general across all
// implementations. These types need to match regardless of backing.

/// For readability. This is a full virt/phys address with page
/// offset. This should be the input and output of most kernel
/// facing functions
pub type Address = usize;

/// A reference to a full page table tree. Likely also an address
/// of some kind.
///
/// It is likely unwise to make this a real rust reference and not
/// a raw address of some kind
pub type PageTable = Address;

/// Things that can go wrong for pgtbl operations
pub enum HALVMError {
    MisalignedAddress,
    FailedAllocation,
    UnsupportedFlags(u32),      // Returns set of unsupported flags
    IgnoredFlags(u32),           // Returns a set of ignored flags
    // TODO others?
}

bitflags! {
/// TODO how do I make this general?
///
/// Things that you can request of a page mapping. Not all may be
/// valid for all hardware. See associated error.
    struct PageMapFlags: u32 {
        const Read     = 0x00_00_00_01;
        const Write    = 0x00_00_00_02;
        const Execute  = 0x00_00_00_04;
        const Valid    = 0x00_00_00_08;
        const User     = 0x00_00_00_10;
        const Global   = 0x00_00_00_20;
        const Accessed = 0x00_00_00_40;
        const Dirty    = 0x00_00_00_80;
    }
}

pub trait HALVM {
    // Page table stuff
    //
    // TODO what are the best types for this kind of stuff? Should I
    // make wrapper types for addresses and pages and whatnot, or
    // should I just use usizes?

    /// same info twice, how big is a page? Number of bytes and number
    /// of offset bits.
    const PAGE_SIZE: usize;
    const OFFSET_BITS: usize;
    // TODO compiletime assert that these match. (1 << OFFSET_BITS) == PAGE_SIZE

    /// Call once before pgtbl use
    fn pgtbl_setup();

    /// Create a new empty page table that can be used with the
    /// following functions.
    fn pgtbl_new_empty() -> Result<PageTable, HALVMError>;

    /// Make a full copy of the supplied page table.
    fn pgtbl_deep_copy(src: PageTable, dest: PageTable) -> Result<(), HALVMError>;

    /// Insert the given page into the given table at the given
    /// location. Flags should be specified here, although it's
    /// totally not clear how to make that general. TODO
    fn pgtbl_insert_leaf(pgtbl: PageTable, phys: Address, virt: Address, flags: PageMapFlags) -> Result<(), HALVMError>;

    /// Remove the mapping at the address in the given page table
    fn pgtbl_remove(pgtbl: PageTable, virt: Address) -> Result<(), HALVMError>;
}

pub trait HALBacking: HALSerial + HALTimer + HALVM {
    /// Run once before any of the rest of the kernel
    fn global_setup();
}


