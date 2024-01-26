//! This module contains the general workings for the riscv paging
//! system, and converts between the various involved types.

pub type ProxyMapper = ();
pub static MAPPER: ProxyMapper = ();

use core::arch::asm;
use alloc::vec;
use crate::alloc::vec::Vec;
use crate::vm::palloc;
use crate::hal::vm::*;

mod ptable;

impl HALVM for ProxyMapper {
    fn kernel_reserved_areas(&self) -> Vec<(PhysPageExtent, PageMapFlags)> {
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

        log!(Warning, "kernel hardware mapping is commented, check if you are getting unexpected behaviors.");

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

    fn kernel_pgtbl_late_setup(&self, pgtbl: &PageTable) {
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

    fn pgtbl_setup(&self) {
        // I don't think I need any global setup. Kernel page table creation happens later.
    }

    fn pgtbl_new_empty(&self) -> Result<PageTable, HALVMError> {
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

    fn pgtbl_deep_copy(&self,
                       _src: PageTable,
                       _dest: PageTable
    ) -> Result<(), HALVMError> {
        todo!("Walk page table and copy as necessary.")
    }

    fn pgtbl_insert_range(&self,
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

    fn pgtbl_remove_range(&self,
                          pgtbl: PageTable,
                          virt: VirtAddress,
                          nbytes: usize) -> Result<(), HALVMError> {
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
        // will allocate intermediate level pages that are filled with
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

    fn pgtbl_swap(&self, pgtbl: &PageTable) {
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

    fn pgtbl_free(&self, _pgtbl: PageTable) {
        todo!("Reuse deep copy code to collect all the pages that have been allocated in the past as intermediate levels of this pagetable.")
    }
}

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
