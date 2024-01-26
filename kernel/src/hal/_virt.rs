/// This module is conditionally included and contains the HAL backing
/// for the qemu riscv 'virt' machine. It assumes that the kernel is
/// booted in S mode by uboot and is running on top of opensbi.

// Useful:
//
// https://github.com/riscv-non-isa/riscv-sbi-doc/blob/master/riscv-sbi.adoc


use core::arch::asm;
use core::ptr::addr_of_mut;
use alloc::vec;

use super::*;
use crate::vm::palloc;

use crate::process::scall_rust_standard;

mod asm;



// -------------------------------------------------------------------
// Trait implementations


// -------------------------------------------------------------------

impl HALTimer for HAL {
    // TODO we need mtime for mtimecmp type stuff here. See the comment in hal.rs
    fn timer_setup() {
        log!(Error, "WE HAVEN'T IMPLEMENTED TIMERS YET!!!");
    }

    fn timer_set(_ticks: u64) {
        todo!("Add timers bucko.")
    }
}

// -------------------------------------------------------------------
mod ptable;

fn flags_hal_to_ptable(general: PageMapFlags) -> Result<usize, HALVMError> {
    let mut out: usize = 0;
    for f in general.into_iter() {
        match f {
            PageMapFlags::Read => {
                out |= ptable::PTE_READ;
            },
            PageMapFlags::Write => {
                out |= ptable::PTE_WRITE;
            },
            PageMapFlags::Execute => {
                out |= ptable::PTE_EXEC;
            },
            PageMapFlags::Valid => {
                out |= ptable::PTE_VALID;
            },
            PageMapFlags::User => {
                out |= ptable::PTE_USER;
            },
            PageMapFlags::Global => {
                out |= ptable::PTE_GLOBAL;
            },
            PageMapFlags::Accessed => {
                out |= ptable::PTE_ACCESSED;
            },
            PageMapFlags::Dirty => {
                out |= ptable::PTE_DIRTY;
            },
            other => {
                return Err(HALVMError::UnsupportedFlags(other));
            }
        }
    }
    return Ok(out);
}

fn table_hal_to_ptable(general: PageTable) -> ptable::PageTable {
    ptable::PageTable {
        base: general.addr,
    }
}

impl HALVM for HAL {
    fn pgtbl_setup() {
        // I don't think I need any global setup. Kernel page table creation happens later.
    }

    fn kernel_reserved_areas() -> Vec<(PhysPageExtent, PageMapFlags)> {
        // It's not clear what of this might / should be handled by opensbi

        const CLINT_BASE: usize = 0x2000000;
        // We don't map the CLINT because we can use opensbi for that
        // (timers). I don't know whether we can/should be mapping the
        // PLIC either.

        const UART_BASE: usize = 0x10000000;
        const UART_SIZE: usize = PAGE_SIZE;

        const PLIC_BASE: usize = 0xc000000;
        const PLIC_SIZE: usize = 0x400000 / PAGE_SIZE;

        const VIRTIO_BASE:usize = 0x10001000;
        const VIRTIO_SIZE: usize = 0x4000 / PAGE_SIZE;

        // TODO one of these mappings is causing issues? I think it
        // might be overruling some opensbi firmware? totally
        // unclear. leave them out until it's clear we need them. I
        // think only PLIC is required, for virtio, as the others are
        // covered by opensbi serial+timers
        vec!(
        //     (PhysPageExtent::new(UART_BASE, PAGE_SIZE), PageMapFlags::Read | PageMapFlags::Write),
        //     (PhysPageExtent::new(PLIC_BASE, PLIC_SIZE), PageMapFlags::Read | PageMapFlags::Write),
        //     (PhysPageExtent::new(VIRTIO_BASE, VIRTIO_SIZE), PageMapFlags::Read | PageMapFlags::Write),
        )
    }


    fn kernel_pgtbl_late_setup(pgtbl: &PageTable) {
        unsafe {
            asm!(
                "csrrw sp, sscratch, sp",
                // space has already been reserved for us, we should write to sp+8
                "sd {page_table}, 8(sp)",
                "csrrw sp, sscratch, sp",
                page_table = in(reg) pgtbl.addr as usize
            );
        }
    }

    /// This call is only valid after other non-hal stuff has been
    /// initialized (page allocation specifically.)
    fn pgtbl_new_empty() -> Result<PageTable, HALVMError> {
        match palloc() {
            Err(_) => {
                Err(HALVMError::FailedAllocation)
            },
            Ok(page) => {
                // palloc should zero for us
                Ok(page)
            }
        }
    }

    fn pgtbl_deep_copy(_src: PageTable,_dest: PageTable) -> Result<(), HALVMError> {
        todo!("Walk page table and copy as necessary.")
    }

    fn pgtbl_insert_range(
        pgtbl: PageTable,
        virt: VirtAddress,
        phys: PhysAddress,
        nbytes: usize,
        flags: PageMapFlags
    ) -> Result<(), HALVMError> {
        // log!(Debug, "{:X} to {:X}", phys as usize, nbytes + phys as usize);
        match ptable::page_map(
            table_hal_to_ptable(pgtbl),
            virt,
            phys,
            nbytes,
            flags_hal_to_ptable(flags)?
        ) {
            Ok(()) => Ok(()),
            Err(_) => Err(HALVMError::FailedAllocation),
        }
    }

    fn pgtbl_remove_range(pgtbl: PageTable, virt: VirtAddress, nbytes: usize) -> Result<(), HALVMError> {
        // TODO add logic for pruning intermediate levels. Currently
        // there is a very slight memory buildup. It's not a leak
        // because they will be cleaned up on free anyway, but we
        // could free them here
        //
        // this leads to an unintuitive feature that removing a
        // mapping can fail due to an out of memory error. This should
        // never happen if all remove_range calls are on intervals
        // that are subsets of previous insert_range calls. Allocation
        // is only required when removing (invalidating) a range that
        // was previously invalid at a higher level, and this function
        // will allocate intermadiate level pages that are filled with
        // invalid entries
        match ptable::page_map(
            table_hal_to_ptable(pgtbl),
            virt,
            0 as PhysAddress,
            nbytes,
            flags_hal_to_ptable(PageMapFlags::empty())?
        ) {
            Ok(()) => Ok(()),
            Err(_) => Err(HALVMError::FailedAllocation),
        }
    }

    fn pgtbl_free(_pgtbl: PageTable) {
        todo!("Reuse deep copy code to collect all the pages that have been allocated in the past as intermediate levels of this pagetable.")
    }

    fn pgtbl_swap(pgtbl: &PageTable) {
        let mut base_addr = pgtbl.addr as usize;
        base_addr = (base_addr >> PAGE_OFFSET) | (8 << 60); // base addr + 39bit addressing
        unsafe {
            asm!(
                "sfence.vma zero, zero",
                "csrw satp, {}",
                "sfence.vma zero, zero",
                in(reg) base_addr
            );
        }
    }
}

// -------------------------------------------------------------------
mod plic;


/// Ideally general handler init for riscv
impl HALIntExc for HAL {
    fn handler_setup() {
        log!(Error, "WE DON'T HAVE UART ACCESS AND SIE IS NOT SET CURRENTLY!!!");
        unsafe {
            asm!(
                // I should clear SIE here and restore later TODO
                "la {hold}, __strapvec",
                "csrw stvec, {hold}",
                hold = out(reg) _,
            );
        }
        plic::global_init();
    }
}

// -------------------------------------------------------------------

impl HALCPU for HAL {
    fn isolate() {
        // This is valid to be empty, as opensbi only starts a single
        // CPU on coldboot, see docs in hal.rs
    }

    fn wake_one<F: Fn() -> !>(_start: F) -> Result<(), HALCPUError> {
        todo!("If you haven't done CPU number discovery, you should do that first.")
    }
}

// -------------------------------------------------------------------
impl HALDiscover for HAL {
    fn discover_setup() {
        log!(Error, "WE ARE USING HARDCODED HARDWARE DISCOVERY!!!");
    }

    const NHART: usize = 2;

    const DRAM_BASE: *mut usize = 0x80000000 as *mut usize;
}



// -------------------------------------------------------------------
mod hartlocal;

impl HALSwitch for HAL {
    type GPInfo = hartlocal::GPInfo;

    fn switch_setup() {
        hartlocal::hartlocal_info_interrupt_stack_init();
    }

    fn save_gp_info(gpi: Self::GPInfo) {
        hartlocal::save_gp_info64(gpi);
    }

    fn restore_gp_info() -> Self::GPInfo {
        hartlocal::restore_gp_info64()
    }
}

// -------------------------------------------------------------------
mod virtio;

impl HALIO for HAL {
    fn io_setup() {
        match virtio::virtio_block_init() {
            Ok(()) => {},
            Err(msg) => {
                panic!("BlockIO init error: {}", msg);
            }
        }
    }

    fn io_barrier() {
        virtio::io_barrier();
    }
}

// -------------------------------------------------------------------

impl HALBacking for HAL {
    fn global_setup() {
        assert!(opensbi_call(BASE_EID, 0, 0, 0, 0, 0).1 == (1<<24) | (0 & 0xFF_FF_FF), "Wrong sbi version");
        Self::serial_setup();
        Self::handler_setup(); // TODO, firgure out how opensbi works with traps
        Self::sections_setup();
        Self::switch_setup();
        Self::timer_setup();
        Self::discover_setup();
        Self::pgtbl_setup();
        Self::io_setup();
    }
}
