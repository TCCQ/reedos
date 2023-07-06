//! Virtual Memory
pub mod global;
pub mod palloc;
pub mod vmalloc;


use alloc::boxed::Box;
use core::alloc::{GlobalAlloc, Layout};
use core::cell::OnceCell;
use core::arch::asm;

use crate::lock::mutex::Mutex;
// use crate::hw::param::*;
use crate::hal::*;
use global::Galloc;
use palloc::*;

// For saftey reasons, no part of the page allocation process all the
// way up past this module can use rust dynamic allocation (global
// usage). This causes a dependency cycle in some places, and a in
// every case opens the possibility of deadlock between the global
// lock and the palloc lock. This is mostly relevant for
// request_phys_page

/// Global physical page pool allocated by the kernel physical allocator.
static mut PAGEPOOL: OnceCell<PagePool> = OnceCell::new();
#[global_allocator]
static mut GLOBAL: GlobalWrapper = GlobalWrapper {
    inner: OnceCell::new(),
};

struct GlobalWrapper {
    inner: OnceCell<Mutex<Galloc>>,
}

unsafe impl GlobalAlloc for GlobalWrapper {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.inner.get().unwrap().lock().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.inner.get().unwrap().lock().dealloc(ptr, layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.inner.get().unwrap().lock().alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        self.inner.get().unwrap().lock().realloc(ptr, layout, new_size)
    }
}

/// (Still growing) list of kernel VM system error cases.
#[derive(Debug)]
pub enum VmError {
    OutOfPages,
    PartialPalloc,
    PallocFail,
    PfreeFail,
    GNoSpace,
    Koom,
}

/// Initialize the kernel VM system.
/// First, setup the kernel physical page pool.
/// We start the pool at the end of the .bss section, and stop at the end of physical memory.
/// Next, we map physical memory into the kernel's physical memory 1:1.
/// Next, initialize the kernel virtual memory allocator pool.
///
/// TODO better error type
pub fn global_init() -> Result<PageTable, ()> {
    unsafe {
        match PAGEPOOL.set(PagePool::new(HAL::bss_end(), HAL::memory_end())) {
            Ok(_) => {}
            Err(_) => {
                panic!("vm double init.")
            }
        }
    }
    log!(Debug, "Successfully initialized kernel page pool...");

    unsafe {
        match GLOBAL.inner.set(Mutex::new(Galloc::new(PAGEPOOL.get_mut().unwrap()))) {
            Ok(_) => {}
            Err(_) => {
                panic!("vm double init.")
            }
        }
    }

    // ---------------------------------------------------------------
    // After this palloc can be used, so we can call HALVM stuff safely

    // Map text, data, stacks, heap into kernel page table.
    match kpage_init() {
        Ok(pt) => {
            return Ok(pt);
        },
        Err(_) => {
            panic!("Failed to setup kernel page table!");
        }
    }
}

pub fn local_init(pt: &PageTable) {
    HAL::pgtbl_swap(pt);
    log!(Info, "WE ARE NOT DOING SECONDARY STACK SETUP YET!!!");
    // pt.write_satp();
    // pagetable_interrupt_stack_setup(pt);
}

// TODO error type?
fn pagetable_interrupt_stack_setup(pt: &PageTable) {
    log!(Debug, "Writing kernel page table {:02X?}", pt.addr);
    unsafe {
        asm!(
            "csrrw sp, sscratch, sp",
            "addi sp, sp, -8",
            "sd {page_table}, (sp)",
            "csrrw sp, sscratch, sp",
            page_table = in(reg) pt.addr as usize
        );
    }
}

/// Create the kernel page table with 1:1 mappings to physical memory.
/// First allocate a new page for the kernel page table.
/// Next, map memory mapped I/O devices to the kernel page table.
/// Then map the kernel .text, .data, .rodata and .bss sections.
/// Additionally, map a stack+guard page for each hart.
/// Finally map, the remaining physical memory to kernel virtual memory as
/// the kernel 'heap'.
pub fn kpage_init() -> Result<PageTable, VmError> {
    let kpage_table = match HAL::pgtbl_new_empty() {
        Ok(p) => p,
        Err(_) => {
            panic!("Could not allocate a kernel page table!");
        }
    };
    // we have acquired the table, now fill it

    // figure out what the hardware wants.

    // this closure lets us handle all of the error sources at once
    let map_pages = || -> Result<(), HALVMError> {
        HAL::pgtbl_insert_range(
            kpage_table,
            HAL::DRAM_BASE,
            HAL::DRAM_BASE as *mut usize,
            HAL::text_end().addr() - HAL::text_start().addr(),
            PageMapFlags::Read | PageMapFlags::Execute
        )?;
        log!(Debug, "Succesfully mapped kernel text into kernel pgtable...");

        HAL::pgtbl_insert_range(
            kpage_table,
            HAL::rodata_start(),
            HAL::rodata_start() as *mut usize,
            HAL::rodata_end().addr() - HAL::rodata_start().addr(),
            PageMapFlags::Read
        )?;
        log!(Debug, "Succesfully mapped kernel rodata into kernel pgtable...");

        HAL::pgtbl_insert_range(
            kpage_table,
            HAL::data_start(),
            HAL::data_start() as *mut usize,
            HAL::data_end().addr() - HAL::data_start().addr(),
            PageMapFlags::Read | PageMapFlags::Write
        )?;
        log!(Debug, "Succesfully mapped kernel data into kernel pgtable...");

        // This maps hart 0, 1 stack pages in opposite order as entry.S. Shouln't necessarily be a
        // problem.
        let base = HAL::stacks_start();
        for s in 0..HAL::NHART {
            let stack = unsafe { base.byte_add(PAGE_SIZE * (1 + s * 3)) };
            HAL::pgtbl_insert_range(
                kpage_table,
                stack,
                stack,
                PAGE_SIZE * 2,
                PageMapFlags::Read | PageMapFlags::Write
            )?;
            log!(
                Debug,
                "Succesfully mapped kernel stack {} into kernel pgtable...",
                s
            );
        }

        // This maps hart 0, 1 stack pages in opposite order as entry.S. Shouln't necessarily be a
        // problem.
        let base = HAL::intstacks_start();
        for i in 0..HAL::NHART {
            let m_intstack = unsafe { base.byte_add(PAGE_SIZE * (1 + i * 4)) };
            // Map hart i m-mode handler.
            HAL::pgtbl_insert_range(
                kpage_table,
                m_intstack,
                m_intstack,
                PAGE_SIZE,
                PageMapFlags::Read | PageMapFlags::Write
            )?;
            // Map hart i s-mode handler
            let s_intstack = unsafe { m_intstack.byte_add(PAGE_SIZE * 2) };
            HAL::pgtbl_insert_range(
                kpage_table,
                s_intstack,
                s_intstack,
                PAGE_SIZE,
                PageMapFlags::Read | PageMapFlags::Write
            )?;
            log!(
                Debug,
                "Succesfully mapped interrupt stack for hart {} into kernel pgtable...",
                i
            );
        }

        HAL::pgtbl_insert_range(
            kpage_table,
            HAL::bss_start(),
            HAL::bss_start(),
            HAL::bss_end().addr() - HAL::bss_start().addr(),
            PageMapFlags::Read | PageMapFlags::Write
        )?;
        log!(Debug, "Succesfully mapped kernel bss...");

        HAL::pgtbl_insert_range(
            kpage_table,
            HAL::bss_end(),
            HAL::bss_end(),
            HAL::memory_end().addr() - HAL::bss_end().addr(),
            PageMapFlags::Read | PageMapFlags::Write
        )?;
        log!(Debug, "Succesfully mapped kernel heap...");

        // finished all generic mappings, now do hardware mappings
        let to_map = HAL::kernel_reserved_areas();
        for (area, flags) in to_map {
            HAL::pgtbl_insert_range(
                kpage_table,
                area.start(),
                area.end(),
                PAGE_SIZE * area.num,
                flags
            )?;
        }
        log!(Debug, "Successfully mapped all hardware specifics...");
        Ok(())
    };

    match map_pages()  {
        Ok(()) => {},
        Err(e) => {
            match e {
                HALVMError::FailedAllocation => {
                    return Err(VmError::PallocFail)
                },
                HALVMError::MisalignedAddress => {
                    panic!("Kernel mapping not page aligned?!")
                },
                HALVMError::UnsupportedFlags(mask) => {
                    panic!("Unsupported flags in kernel mapping: {mask:x}!");
                }
            }
        },
    }
    Ok(kpage_table)
}

/// A test designed to be used with GDB.
/// Allocate A, then B. Free A, then B.
pub unsafe fn test_palloc() {
    let one = PAGEPOOL.get_mut().unwrap().palloc().unwrap();
    one.addr.write(0xdeadbeaf);

    let many = PAGEPOOL.get_mut().unwrap().palloc_plural(5).unwrap();
    many.write_bytes(5, 512 * 2);

    let _ = PAGEPOOL.get_mut().unwrap().pfree(one);
    let _ = PAGEPOOL.get_mut().unwrap().pfree_plural(many, 5);

    log!(Debug, "Successful test of page allocation and freeing...");
}

pub unsafe fn test_galloc() {
    use alloc::collections;
    {
        // Simple test. It works!
        let mut one = Box::new(5);
        let a_one: *mut u32 = one.as_mut();
        assert_eq!(*one, *a_one);

        // Slightly more interesting... it also works! Look at GDB
        // and watch for the zone headers + chunk headers indicating 'in use' and
        // 'chunk size'. Then watch as these go out of scope.
        let mut one_vec: Box<collections::VecDeque<u32>> = Box::default();
        one_vec.push_back(555);
        one_vec.push_front(111);
        let _a_vec: *mut collections::VecDeque<u32> = one_vec.as_mut();
    }

    log!(Debug, "Successful test of alloc crate...");
}

// -------------------------------------------------------------------


// /// See `vm::vmalloc::Kalloc::alloc`.
// pub fn kalloc(size: usize) -> Result<*mut usize, vmalloc::KallocError> {
//     unsafe { VMALLOC.get_mut().unwrap().alloc(size) }
// }

// /// See `vm::vmalloc::Kalloc::free`.
// pub fn kfree<T>(ptr: *mut T) {
//     unsafe { VMALLOC.get_mut().unwrap().free(ptr) }
// }

// exposed, but request_phys_page is preferred
pub fn palloc() -> Result<Page, VmError> {
    unsafe { PAGEPOOL.get_mut().unwrap().palloc() }
}

pub fn pfree(page: Page) -> Result<(), VmError> {
    unsafe { PAGEPOOL.get_mut().unwrap().pfree(page) }
}


// -------------------------------------------------------------------

// TODO consider discovery mechanism to test if allocation is up yet

/// Out facing interface for physical pages. Automatically cleaned up
/// on drop. Intentionally does not impliment clone/copy/anything.
pub struct PhysPageExtent {
    head: Page,
    num: usize,
}

impl PhysPageExtent {
    pub fn new(head: usize, num: usize) -> Self {
        Self {
            head: Page { addr: head as *mut usize},
            num,
        }
    }

    pub fn start(&self) -> *mut usize {
        self.head.addr
    }

    pub fn end(&self) -> *mut usize {
        unsafe {
            self.head.addr.byte_add(self.num * PAGE_SIZE)
        }
    }
}

impl Drop for PhysPageExtent {
    fn drop(&mut self) {
        unsafe {
            match PAGEPOOL.get_mut().unwrap()
                .pfree_plural(self.head.addr, self.num) {
                    Ok(_) => {},
                    Err(e) => {panic!("Double palloc free! {:?}", e)}
            }
        }
    }
}

unsafe impl Send for PhysPageExtent {}


// VERY IMPORTANT: see top of module comment about deadlock safety
/// Should be one and only way to get physical pages outside of vm module/subsystem.
pub fn request_phys_page(num: usize) -> Result<PhysPageExtent, VmError>{
    let addr = unsafe {
        PAGEPOOL.get_mut().unwrap().palloc_plural(num)?
    };
    Ok(PhysPageExtent {
        head: Page::from(addr),
        num,
    })
}

pub fn test_phys_page() {
    {
        let _ = request_phys_page(1).unwrap();
        let _ = request_phys_page(2).unwrap();
    }
    let _ = request_phys_page(1).unwrap();
}
