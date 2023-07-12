use alloc::vec::Vec;
/// This module should contain the details of the hardware abstraction
/// layer

use bitflags::bitflags;

use crate::vm::{palloc::Page, PhysPageExtent};

#[cfg(feature = "hal-virt")]
pub mod virt;

pub struct HAL {}

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

// -------------------------------------------------------------------
// Virtual memory stuff

/// For readability. This is a full virt/phys address with page
/// offset. This should be the input and output of most kernel
/// facing functions
pub type VirtAddress = *mut usize;
pub type PhysAddress = *mut usize;

/// A reference to a full page table tree. Likely also an address
/// of some kind.
///
/// It is likely unwise to make this a real rust reference and not
/// a raw address of some kind
pub type PageTable = Page;

/// Things that can go wrong for pgtbl operations
pub enum HALVMError {
    MisalignedAddress,
    FailedAllocation,
    UnsupportedFlags(PageMapFlags),      // Returns set of unsupported flags
    // TODO others?
}

bitflags! {
/// TODO how do I make this general?
///
/// Things that you can request of a page mapping. Not all may be
/// valid for all hardware. See associated error.
    #[derive(PartialEq, Eq)]
    pub struct PageMapFlags: u32 {
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

pub const PAGE_SIZE: usize = 4096;
pub const PAGE_OFFSET: usize = 12;

pub trait HALVM {
    // Page table stuff

    /// Return a set of memory regions that should be mapped into the
    /// kernel page table with the given permissions. It is an error
    /// to call this before starting allocation in the `vm` module. It
    /// is an error to call this before `pgtbl_setup`. It is the
    /// implementer of HALVM's responsibility to ensure that there are
    /// not overlaps with the generic kernel mappings. Hardware
    /// specific mappings will overwrite general mappings
    fn kernel_reserved_areas() -> Vec<(PhysPageExtent, PageMapFlags)>;

    /// Called on the populated kernel pagetable. May be empty for
    /// some platforms
    fn kernel_pgtbl_late_setup(pgtbl: &PageTable);

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
    fn pgtbl_insert_range(pgtbl: PageTable, virt: VirtAddress, phys: PhysAddress, nbytes: usize, flags: PageMapFlags) -> Result<(), HALVMError>;

    /// Remove the mapping at the address in the given page table
    fn pgtbl_remove_range(pgtbl: PageTable, virt: VirtAddress, nbytes: usize) -> Result<(), HALVMError>;

    /// Change your page table. Only safe in the next instruction
    /// (probably a whole bunch of text, including this function and
    /// whatever caller you need to direct traffic) is mapped with
    /// appropriate permissions in destination page table.
    fn pgtbl_swap(pgtbl: &PageTable);

    // TODO make this a drop trait
    fn pgtbl_free(pgtbl: PageTable);
}

// -------------------------------------------------------------------
//

/// This should conceptually contain all the stuff related to
/// interupts and exceptions. Since those are so deeply hardware
/// dependant, this trait will only require a single setup function,
/// that should do whatever installation and set up is necessary for
/// the hardware in question. The handlers that get installed can call
/// out to the main kernel, but the implementer of the HAL is
/// responsible for ensuring that the calls and their side effects are
/// safe for the current execution environement (Priviledge, Current
/// page table, type of handler). A natural ideal point for
/// generalization would be the syscalls, but even those calling
/// conventions are different. The syscall main handler will continue
/// to exist outside of the HAL, but small hardware specific changes
/// can be implemented with addributes that check for the cargo
/// features for the desired HAl.
pub trait HALIntExc {
    /// This function should set up and install all interupt and
    /// exception handlers required by the system. It does not return
    /// any errors, and instead should panic on error.
    fn handler_setup();
}

// -------------------------------------------------------------------
// CPU and executor control

pub enum HALCPUError {
    OutOfCPU,
}

pub trait HALCPU {
    /// Call by all cpus, a single one will return. Others can be
    /// retrieved later with wake_one. This call should be used before
    /// any setup, and is only valid to call a single time. If a
    /// platform or hardware guarantees that only one CPU will wake on
    /// a cold boot, this call can be empty, however wake_one should
    /// still function as expected. (This is the case for opensbi).
    ///
    /// If this call cannot succeed, it should panic rather than
    /// return an error.
    ///
    /// It is valid to call this before global_setup is called.
    fn isolate();

    /// Retrieve some CPU from the isolate call. This call is only
    /// valid after the single textural call to isolate. It will
    /// return an error or nothing to the caller depending on if the
    /// wakeup was successful. A platform should have some other way
    /// of determing the number of CPUs. If a CPU is woken, it's
    /// execution starts at the passed function.
    ///
    /// This should probably only be called after global_setup
    fn wake_one<F: Fn() -> !>(start: F) -> Result<(), HALCPUError>;
}

// -------------------------------------------------------------------

/// This wraps all hardware discovery.
///
/// TODO when we do device tree stuff, it will be exposed here
pub trait HALDiscover {
    fn discover_setup();

    // Const in the short term, but we will see
    const NHART: usize;

    // I think this is safe to be a const. It can change when we have
    // reason for it.
    const DRAM_BASE: *mut usize;
}

// -------------------------------------------------------------------

/// Provide info about the sections of the kernel. This trait must be
/// be safe to call before allocation is brought up.
pub trait HALSections {
    /// This may very well be empty
    fn sections_setup();

    fn text_start() -> *mut usize;
    fn text_end() -> *mut usize;

    fn rodata_start() -> *mut usize;
    fn rodata_end() -> *mut usize;

    fn data_start() -> *mut usize;
    fn data_end() -> *mut usize;

    fn stacks_start() -> *mut usize;
    fn stacks_end() -> *mut usize;

    fn intstacks_start() -> *mut usize;
    fn intstacks_end() -> *mut usize;

    fn bss_start() -> *mut usize;
    fn bss_end() -> *mut usize;

    // TODO deprecate? wait for clarity as for context switches under
    // opensbi
    fn memory_start() -> *mut usize;
    fn memory_end() -> *mut usize;
}


// -------------------------------------------------------------------

/// Trait the collect context switching stuff
pub trait HALSwitch {
    /// This type should contain the initial info that needs to be
    /// presereved through a context switch. This should not include
    /// things like the kernel page table which from the point of view
    /// of the main kernel should be switched automatically, but
    /// should contian things like the current process. Think of it
    /// like the arguments to the first function after a context
    /// switch.
    ///
    /// The implementation should provide a method of creating a
    /// GPInfo structure, including a Process object to represent the
    /// currently executing process.
    type GPInfo;

    /// called once before any of the switching occurs, just like the
    /// rest.
    fn switch_setup();

    //GPInfo management is CPU local

    /// Set the structure for the next restore.
    fn save_gp_info(gpi: Self::GPInfo);

    /// Restore the most recently saved structure.
    fn restore_gp_info() -> Self::GPInfo;

    // TODO
    //
    // I think it makes sense for the unsafe / extern C boundary into
    // the asm to be in the hal, so the main kernel just sees a safe
    // never returning call, but I don't think the signature can be
    // any more general than pc, pgtbl, sp. Porters can bring up
    // issues if there are any later. Also you can't enforce the
    // implementation of extern C functions in a trait, so what's the
    // point.
    //
    // In spirit, here we need to make sure *somewhere* there is a
    //
    // extern "C" {pub fn process_resume_asm(pc: usize, pgtbl: usize, sp: usize) -> !;}
    // extern "C" {pub fn process_start_asm(pc: usize, pgtbl: usize, sp: usize) -> !;}
    // extern "C" {pub fn process_exit_rust(exit_code: isize) -> !;}
    //
    // implemented for every backing.
}

pub trait HALBacking:
HALSerial + HALTimer +
    HALVM + HALIntExc +
    HALCPU + HALDiscover +
    HALSections + HALSwitch
{
    /// Call on all CPUs on start, a single one will exit, and all others will hold, until a later wakeup call

    /// Run once before any of the rest of the kernel
    fn global_setup();
}


