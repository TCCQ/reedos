//! Page table
// VA: 39bits, PA: 56bits
// PTE size = 8 bytes
// use crate::hw::riscv::*;

use core::assert;
use core::arch::asm;

use crate::vm::*;
use crate::hal::*;                   // virt/hal stuff

pub const VA_TOP: usize = 1 << (27 + 12); // 2^27 VPN + 12 Offset
pub const PTE_TOP: usize = 512; // 4Kb / 8 byte PTEs = 512 PTEs / page!
pub const PTE_VALID: usize = 1 << 0;
pub const PTE_READ: usize = 1 << 1;
pub const PTE_WRITE: usize = 1 << 2;
pub const PTE_EXEC: usize = 1 << 3;
pub const PTE_USER: usize = 1 << 4;
pub const PTE_GLOBAL: usize = 1 << 5;
pub const PTE_ACCESSED: usize = 1 << 6;
pub const PTE_DIRTY: usize = 1 << 7;

type PTEntry = usize;
/// Supervisor Address Translation and Protection.
/// Section 4.1.12 of risc-v priviliged ISA manual.
type SATPAddress = usize;

/// Abstraction of a page table at a physical address.
/// Notice we didn't use a rust array here, instead
/// implementing our own indexing methods, functionally
/// similar to that of an array.
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTable {
    pub base: PhysAddress, // Page Table located at base address.
}

#[inline(always)]
fn vpn(ptr: VirtAddress, level: usize) -> usize {
    ptr.addr() >> (12 + 9 * level) & 0x1FF
}

#[inline(always)]
fn pte_to_phy(pte: PTEntry) -> PhysAddress {
    ((pte >> 10) << 12) as *mut usize
}

#[inline(always)]
fn phy_to_pte(ptr: PhysAddress) -> PTEntry {
    ((ptr.addr()) >> 12) << 10
}

macro_rules! PteGetFlag {
    ($pte:expr, $flag:expr) => {
        ($pte) & $flag != 0
    };
}

macro_rules! PteSetFlag {
    ($pte:expr, $flag:expr) => {
        (($pte) | $flag)
    };
}

#[inline(always)]
fn phy_to_satp(ptr: PhysAddress) -> usize {
    (1 << 63) | (ptr.addr() >> 12)
}

macro_rules! PageAlignDown {
    ($p:expr) => {
        ($p).map_addr(|addr| addr & !(PAGE_SIZE - 1))
    };
}

// Read the memory at location self + index * 8 bytes
unsafe fn get_phy_offset(phy: PhysAddress, index: usize) -> *mut PTEntry {
    phy.byte_add(index * 8)
}

fn set_pte(pte: *mut PTEntry, contents: PTEntry) {
    unsafe {
        pte.write_volatile(contents);
    }
}

fn read_pte(pte: *mut PTEntry) -> PTEntry {
    unsafe { pte.read_volatile() }
}

impl From<PTEntry> for PageTable {
    fn from(pte: PTEntry) -> Self {
        PageTable {
            base: pte_to_phy(pte),
        }
    }
}

// stolen from riscv.rs
pub fn flush_tlb() {
    unsafe {
        asm!("sfence.vma zero, zero");
    }
}
pub fn write_satp(pt: usize) {
    unsafe {
        asm!("csrw satp, {}", in(reg) pt);
    }
}

impl PageTable {
    pub fn new(addr: *mut usize) -> Self {
        Self {
            base: addr as PhysAddress,
        }
    }

    fn index_mut(&self, idx: usize) -> *mut PTEntry {
        assert!(idx < PTE_TOP);
        unsafe { get_phy_offset(self.base, idx) }
    }
    pub fn write_satp(&self) {
        flush_tlb();
        write_satp(phy_to_satp(self.base));
        flush_tlb();
    }
}

// Get the address of the PTE for va given the page table pt.
// Returns Either PTE or None, callers responsibility to use PTE
// or allocate a new page.
unsafe fn walk(pt: PageTable, va: VirtAddress, alloc_new: bool) -> Result<*mut PTEntry, VmError> {
    let mut table = pt;
    assert!(va.addr() < VA_TOP);
    for level in (1..3).rev() {
        let idx = vpn(va, level);
        let next: *mut PTEntry = table.index_mut(idx);
        table = match PteGetFlag!(*next, PTE_VALID) {
            true => PageTable::from(*next),
            false => {
                if alloc_new {
                    match palloc() {
                        Ok(pg) => {
                            *next = PteSetFlag!(phy_to_pte(pg.addr), PTE_VALID);
                            PageTable::from(phy_to_pte(pg.addr))
                        }
                        Err(e) => return Err(e),
                    }
                } else {
                    return Err(VmError::PallocFail);
                }
            }
        };
    }
    // Last, return PTE leaf. Assuming we are all using 4K pages right now.
    // Caller's responsibility to check flags.
    let idx = vpn(va, 0);
    Ok(table.index_mut(idx))
}

/// Maps some number of pages into the VM given by pt of byte length
/// size.
pub fn page_map(
    pt: PageTable,
    va: VirtAddress,
    pa: PhysAddress,
    size: usize,
    flag: usize,
) -> Result<(), VmError> {
    // Round down to page aligned boundary (multiple of pg size).
    let mut start = PageAlignDown!(va);
    let mut phys = pa;
    let end = PageAlignDown!(va.map_addr(|addr| addr + (size - 1)));

    while start <= end {
        let walk_addr = unsafe { walk(pt, start, true) };
        match walk_addr {
            Err(e) => {
                return Err(e);
            }
            Ok(pte_addr) => {
                if read_pte(pte_addr) & PTE_VALID != 0 {
                    return Err(VmError::PallocFail);
                }
                set_pte(pte_addr, PteSetFlag!(phy_to_pte(phys), flag | PTE_VALID));
                start = start.map_addr(|addr| addr + PAGE_SIZE);
                phys = phys.map_addr(|addr| addr + PAGE_SIZE);
            }
        }
    }

    Ok(())
}
