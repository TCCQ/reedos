/// This module is conditionally included and contains the HAL backing
/// for the qemu riscv 'virt' machine. It assumes that the kernel is
/// booted in S mode by uboot and is running on top of opensbi.

// Useful:
//
// https://github.com/riscv-non-isa/riscv-sbi-doc/blob/master/riscv-sbi.adoc

// TODO
//
// add failstate logging, for use with panics. (i.e. write the most
// recent panic message to a fixed point in memory so we can read it
// with a debugger if that issue is so bad that our printing doesn't
// work) Not a priority

use core::arch::asm;
use core::ptr::addr_of_mut;
use alloc::vec;

use super::*;
use crate::vm::palloc;

use crate::process::scall_rust_standard;

mod asm;

// -------------------------------------------------------------------
// Shim for misc bused riscv stuff

#[no_mangle]
pub fn fmod(_a: f64, _b: f64) -> f64 {
    todo!("No fmod support for riscv?");
}

#[no_mangle]
pub fn fmodf(_a: f32, _b: f32) -> f32 {
    todo!("No fmod support for riscv?");
}


// -------------------------------------------------------------------
// Opensbi stuff

const DEBUG_EID: u32 = 0x4442434E;
const BASE_EID: u32 = 0x10;

const SBI_SUCCESS: i32               =  0; // Completed successfully
const SBI_ERR_FAILED: i32            = -1; // Failed
const SBI_ERR_NOT_SUPPORTED: i32     = -2; // Not supported
const SBI_ERR_INVALID_PARAM: i32     = -3; // Invalid parameter(s)
const SBI_ERR_DENIED: i32            = -4; // Denied or not allowed
const SBI_ERR_INVALID_ADDRESS: i32   = -5; // Invalid address(s)
const SBI_ERR_ALREADY_AVAILABLE: i32 = -6; // Already available
const SBI_ERR_ALREADY_STARTED: i32   = -7; // Already started
const SBI_ERR_ALREADY_STOPPED: i32   = -8; // Already stopped
const SBI_ERR_NO_SHMEM: i32          = -9; // Shared memory not available

fn _opensbi_call(eid: usize, fid: usize, mut a0: usize, mut a1: usize, a2: usize, a3: usize) -> (i32, u32) {
    unsafe {
        asm!(
            "ecall",
            in("a7") eid,
            in("a6") fid,
            inout("a0") a0,
            inout("a1") a1,
            in("a2") a2,
            in("a3") a3,
        );
    }
    (a0 as i32, a1 as u32)
}

fn opensbi_call(eid: u32, fid: u32, a0: u32, a1: u32, a2: u32, a3: u32) -> (i32, u32) {
    _opensbi_call(eid as usize, fid as usize, a0 as usize, a1 as usize, a2 as usize, a3 as usize)
}

// -------------------------------------------------------------------
// Trait implementations

impl HALSerial for HAL {
    fn serial_setup() {
        // probe for opensbi debug console extension
        let (err, val) = opensbi_call(BASE_EID, 3, DEBUG_EID, 0, 0, 0);
        match err {
            0 => {
                // all good!
                match val {
                    0 => {
                        panic!("Opensbi does not support debug logging!");
                    },
                    1 => {
                        // exactly as we want
                    },
                    _ => {
                        panic!("Unexpected opensbi return code!");
                    }
                }
            },
            SBI_ERR_FAILED | SBI_ERR_NOT_SUPPORTED | SBI_ERR_DENIED => {
                panic!("Unsupported base opensbi extension!");
            },
            _ => {
                panic!("Unexpected opensbi error code!");
            }
        }
    }

    fn serial_put_char(c: char) {
        let mut buffer: [u8; 4] = [0; 4];
        let slice = c.encode_utf8(&mut buffer);
        for iter in slice.as_bytes() {
            let val: u8 = iter.clone();
            // opensbi console putchar
            let (err, _) = opensbi_call(DEBUG_EID, 2, val as u32, 0, 0, 0);
            match err {
                0 => {},
                SBI_ERR_FAILED => {
                    panic!("Failed to write to console!");
                },
                _ => {
                    panic!("Unexpected opensbi error code!");
                }
            }
        }
    }

    fn serial_read_byte() -> u8 {
        let mut val: u8 = 0;
        // opensbi console getchar
        let (err, _ret) = opensbi_call(DEBUG_EID, 1,
                     1,         // 1 byte
                     ((&mut val as *mut u8 as usize) & 0xFF_FF_FF_FF) as u32, // low bits
                     ((&mut val as *mut u8 as usize) >> 32) as u32,                  // high bits
                     0,
        );
        match err {
            SBI_SUCCESS => {},
            SBI_ERR_FAILED => {
                panic!("Opensbi I/O fail on read");
            }
            SBI_ERR_INVALID_PARAM => {
                panic!("Opensbi didn't like arguments to console read");
            },
            _ => {
                panic!("Unexpected opensbi error code");
            }
        }
        val
    }

    fn serial_put_string(s: &str) {
        let (err, _ret) = opensbi_call(DEBUG_EID, 0,
                     s.len() as u32,         // 1 byte
                     ((s.as_ptr() as usize) & 0xFF_FF_FF_FF) as u32, // low bits
                     ((s.as_ptr() as usize) >> 32) as u32,                  // high bits
                     0,
        );
        match err {
            SBI_SUCCESS => {},
            SBI_ERR_FAILED => {
                panic!("Opensbi I/O fail on write");
            }
            SBI_ERR_INVALID_PARAM => {
                panic!("Opensbi didn't like arguments to console write");
            },
            _ => {
                panic!("Unexpected opensbi error code");
            }
        }

    }

    // TODO pass errors back up
    fn serial_read_bytes(buf: &mut [u8], num: u32) {
        let (err, _ret) = opensbi_call(DEBUG_EID, 1,
                     num,         // 1 byte
                     ((buf.as_mut_ptr() as usize) & 0xFF_FF_FF_FF) as u32, // low bits
                     ((buf.as_mut_ptr() as usize) >> 32) as u32,                  // high bits
                     0,
        );
        match err {
            SBI_SUCCESS => {},
            SBI_ERR_FAILED => {
                panic!("Opensbi I/O fail on read");
            }
            SBI_ERR_INVALID_PARAM => {
                panic!("Opensbi didn't like arguments to console read");
            },
            _ => {
                panic!("Unexpected opensbi error code");
            }
        }

    }
}

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

fn read_scause() -> usize {
    unsafe {
        let out: usize;
        asm!(
            "csrr {out}, scause",
            out = out(reg) out
        );
        out
    }
}

fn read_stval() -> usize {
    unsafe {
        let out: usize;
        asm!(
            "csrr {out}, stval",
            out = out(reg) out
        );
        out
    }
}

/// These are the cause numbers for the regular s mode handler. I don't
/// see any reason they need to be public.
///
/// TODO how can we make these generic over 32/64 bit width?
const S_EXTERN_IRQ: usize = 0x9 | ( 1 << 63);
const S_STORE_AMO_FAULT: usize = 0xf;
const S_LOAD_PAGE_FAULT: usize = 0xd;

/// Supervisor mode trap handler.
#[no_mangle]
pub extern "C" fn s_handler() {
    let cause = read_scause();

    match cause {
        S_EXTERN_IRQ => {
            s_extern()
        },
        S_STORE_AMO_FAULT => {
            // This is a write page fault (or a kind of write permission fault)

            let val = read_stval();

            // We want to catch stack over/underflow specifically;
            if val >= HAL::stacks_start() as usize &&
                val < (HAL::stacks_end() as usize + PAGE_SIZE) {
                    // error with the stack area. Hit a guard page

                    // TODO currently we can't tell if this is an over
                    // or an underflow, because we don't know which
                    // stack we were on originally. Our current HAL
                    // does not allow for known CPU ids. This could be
                    // changed, but I like the anonymity frankly
                    panic!("Stack over or underflow. Make sure you don't have a huge stack frame somewhere, or cut some recursion!");
                } else {
                    panic!("Store/AMO fault. Faulting address 0x{:x}", val);
                }
        },
        S_LOAD_PAGE_FAULT => {
            // This is a read page fault

            let val = read_stval();
            panic!("Load page fault. Faulting address 0x{:x}", val);
        },
        _ => {
            log!(
                Warning,
                "Uncaught supervisor mode interupt. scause: 0x{:x}",
                cause
            );
            panic!("s_handler panic")
        }
    }
}

/// Called when we get a S mode external interupt. Probably UART input
/// or virtio.
fn s_extern() {
    let irq = unsafe {
        plic::PLIC.get().expect("PLIC not initialized!").claim()
    };

    const UART_IRQ: u32 = plic::UART_IRQ as u32;
    const VIRTIO_IRQ: u32 = plic::VIRTIO_IRQ as u32;
    match irq {
        0 => {
            // reserved for "No interrupt" according to the
            // cookbook. Just chill I guess, I don't think we need to
            // complete it
        }
        UART_IRQ => {
            // I intentionally don't hold the lock here to
            // allow printing. Normally we shouldn't print
            // here


            // TODO figure out how UART input works with opensbi. It's
            // just whatever the opensbi getChar is right? Is there a
            // non-blocking version, or an interrupting version? does
            // it have buffering?

            panic!("Unexpected UART input interrupt. Are you not using opensbi?");
        },
        VIRTIO_IRQ => {
            // TODO I am assuming blindly that virtio works as normal
            // under opensbi. We can dump PLIC registers on virt to
            // figure that out I guess.
            virtio::virtio_blk_intr();
            unsafe {
                plic::PLIC.get().unwrap().complete(irq)
            };
        },
        _ => {
            panic!("Uncaught PLIC exception.")
        }
    }
}

extern "C" {
    pub fn __mtrapvec();
    pub fn __strapvec();
}

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
// These are supplied by the linkerscript at the moment and is thus
// stateless, fully compiletime, and does not require device tree
// parsing or any other kind of discovery. This method does assume
// that the memory placement is known at compile time (seems
// reasonable for a kernel), and that the linkerscript assumptions
// about memory are accurate. Thus at the moment no one compilation
// will work for multiple boards. But at the moment it's a good
// middlground.

// This ugly two macro setup is gross, but I can't use just one to do
// both cause each invocation would need to strattle both. And you
// can't split impls so there is no workaround here.
macro_rules! linker_var {
    (
        $linker_name: ident
    ) => {
        extern "C" { static mut $linker_name: usize; }
    }
}

macro_rules! trait_wrapper {
    (
        $linker_name: ident,
        $rust_name: ident
    ) => {
        #[doc="Get the associated linker variable as a pointer"]
        fn $rust_name() -> *mut usize {
            unsafe { addr_of_mut!($linker_name) }
        }
    }
}

linker_var!(_text_start);
linker_var!(_text_end);

linker_var!(_bss_start);
linker_var!(_bss_end);

linker_var!(_rodata_start);
linker_var!(_rodata_end);

linker_var!(_data_start);
linker_var!(_data_end);

linker_var!(_stacks_start);
linker_var!(_stacks_end);

linker_var!(_intstacks_start);
linker_var!(_intstacks_end);

linker_var!(_memory_start);
linker_var!(_memory_end);

impl HALSections for HAL {
    fn sections_setup() {
        // This is intentionally empty
    }

    trait_wrapper!(_text_start, text_start);
    trait_wrapper!(_text_end, text_end);

    trait_wrapper!(_bss_start, bss_start);
    trait_wrapper!(_bss_end, bss_end);

    trait_wrapper!(_rodata_start, rodata_start);
    trait_wrapper!(_rodata_end, rodata_end);

    trait_wrapper!(_data_start, data_start);
    trait_wrapper!(_data_end, data_end);

    trait_wrapper!(_stacks_start, stacks_start);
    trait_wrapper!(_stacks_end, stacks_end);

    trait_wrapper!(_intstacks_start, intstacks_start);
    trait_wrapper!(_intstacks_end, intstacks_end);

    trait_wrapper!(_memory_start, memory_start);
    trait_wrapper!(_memory_end, memory_end);
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

#[no_mangle]
pub extern "C" fn scall_rust(a0: usize, a1: usize, a2: usize, a3: usize,
                             a4: usize, a5: usize, a6: usize, a7: usize)
                             {
    let proc_pc: usize;
    let proc_sp: usize;
    unsafe {
        asm!(
            "mv {pc}, s2",
            "mv {sp}, s3",
            pc = out(reg) proc_pc,
            sp = out(reg) proc_sp
        );
    }
    scall_rust_standard(a0,a1,a2,a3,a4,a5,a6,a7, proc_pc, proc_sp)
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
