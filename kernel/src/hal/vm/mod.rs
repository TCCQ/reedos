//! This module presents the platform agnostic view of virtual memory.

use bitflags::bitflags;

use alloc::vec::Vec;
use crate::vm::{palloc::Page, PhysPageExtent};

// -------------------------------------------------------------------
//
// Public interface

/// For readability. This is a full virt/phys address with page
/// offset. This should be the input and output of most kernel
/// facing functions
pub type VirtAddress = *mut usize;
pub type PhysAddress = *mut usize;

/// A reference to a full page table tree. Likely also an address
/// of some kind.
pub type PageTable = Page;
// TODO this introduces a type dep outside of the hal module tree.

/// Things that can go wrong for pgtbl operations
pub enum HALVMError {
    MisalignedAddress,
    FailedAllocation,
    UnsupportedFlags(PageMapFlags),      // Returns set of unsupported flags
    // TODO others?
}

bitflags! {
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
// TODO hardcoded?

/// Return a set of memory regions that should be mapped into the
/// kernel page table with the given permissions. It is an error
/// to call this before starting allocation in the `vm` module. It
/// is an error to call this before `pgtbl_setup`. It is the
/// implementer of HALVM's responsibility to ensure that there are
/// not overlaps with the generic kernel mappings. Hardware
/// specific mappings will overwrite general mappings
pub fn kernel_reserved_areas() -> Vec<(PhysPageExtent, PageMapFlags)> {
    backing.kernel_reserved_areas()
}

/// To be called on the populated kernel pagetable. May be empty for
/// some platforms
pub fn kernel_pgtbl_late_setup(pgtbl: &PageTable) {
    backing.kernel_pgtbl_late_setup(pgtbl)
}

/// Call once before pgtbl use
pub fn pgtbl_setup() {
    backing.pgtbl_setup()
}

/// Create a new empty page table that can be used with the
/// following functions.
pub fn pgtbl_new_empty() -> Result<PageTable, HALVMError> {
    backing.pgtbl_new_empty()
}

/// Make a full copy of the supplied page table.
pub fn pgtbl_deep_copy(src: PageTable, dest: PageTable) -> Result<(), HALVMError> {
    backing.pgtbl_deep_copy(src, dest)
}

/// Insert the given page into the given table at the given
/// location. Flags should be specified here, although it's
/// totally not clear how to make that general.
pub fn pgtbl_insert_range(
    pgtbl: PageTable,
    virt: VirtAddress,
    phys: PhysAddress,
    nbytes: usize,
    flags: PageMapFlags
) -> Result<(), HALVMError> {
    backing.pgtbl_insert_range(pgtbl, virt, phys, nbytes, flags)
}

/// Remove the mapping at the address in the given page table
pub fn pgtbl_remove_range(pgtbl: PageTable, virt: VirtAddress, nbytes: usize) -> Result<(), HALVMError> {
    backing.pgtbl_remove_range(pgtbl, virt, nbytes)
}

/// Change your page table. Only safe if the next instruction
/// (probably a whole bunch of text, including this function and
/// whatever caller you need to direct traffic) is mapped with
/// appropriate permissions in destination page table.
pub fn pgtbl_swap(pgtbl: &PageTable) {
    backing.pgtbl_swap(pgtbl)
}

// TODO make this a drop trait. Will that ruin inheritence?
pub fn pgtbl_free(pgtbl: PageTable) {
    backing.pgtbl_free(pgtbl)
}

// -------------------------------------------------------------------
//
// Backend selection

#[cfg(feature = "riscv")]
mod riscv;

#[cfg(feature = "riscv")]
static backing: riscv::ProxyMapper = riscv::MAPPER;

// -------------------------------------------------------------------
//
// Traits to talk to backends

pub trait HALVM {
    fn kernel_reserved_areas(&self) -> Vec<(PhysPageExtent, PageMapFlags)>;

    fn kernel_pgtbl_late_setup(&self, pgtbl: &PageTable);

    fn pgtbl_setup(&self);

    fn pgtbl_new_empty(&self) -> Result<PageTable, HALVMError>;

    fn pgtbl_deep_copy(&self, src: PageTable, dest: PageTable) -> Result<(), HALVMError>;

    fn pgtbl_insert_range(&self,
        pgtbl: PageTable,
        virt: VirtAddress,
        phys: PhysAddress,
        nbytes: usize,
        flags: PageMapFlags
    ) -> Result<(), HALVMError>;

    fn pgtbl_remove_range(&self, pgtbl: PageTable, virt: VirtAddress, nbytes: usize) -> Result<(), HALVMError>;

    fn pgtbl_swap(&self, pgtbl: &PageTable);

    fn pgtbl_free(&self, pgtbl: PageTable);
}
