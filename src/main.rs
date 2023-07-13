//! minimal rust kernel built for (qemu virt machine) riscv.
#![no_std]
#![no_main]
#![feature(pointer_byte_offsets)]
#![feature(error_in_core)]
#![feature(sync_unsafe_cell)]
#![feature(panic_info_message)]
#![feature(strict_provenance)]
#![feature(unsized_fn_params)]
#![feature(box_into_inner)]
#![feature(never_type)]
#![feature(lazy_cell)]
#![allow(dead_code)]

use core::cell::OnceCell;
use core::mem::MaybeUninit;
use core::panic::PanicInfo;
extern crate alloc;

#[macro_use]
pub mod log;
// ^ has to come first cause of ordered macro scoping

pub mod device;
pub mod lock;
pub mod vm;
pub mod process;
pub mod file;
pub mod hal;
pub mod id;

pub static BANNER: &str = r#"
Mellow Swirled,
                       __
   ________  ___  ____/ /___  _____
  / ___/ _ \/ _ \/ __  / __ \/ ___/
 / /  /  __/  __/ /_/ / /_/ (__  )
/_/   \___/\___/\__,_/\____/____/

"#;

use crate::lock::condition::ConditionVar;
use crate::hal::*;

// sync init accross harts
static mut GLOBAL_INIT_FLAG: MaybeUninit<ConditionVar> = MaybeUninit::uninit();
// pass the initial kernel page table to non-zero id harts. This is
// not how it is accessed after inialization
static mut KERNEL_PAGE_TABLE: OnceCell<PageTable> = OnceCell::new();

// The never type "!" means diverging function (never returns).
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let default = format_args!("No message provided");
    let msg = match info.message() {
        Some(msg) => msg,
        None => &default,
    };
    match info.location() {
        None => {
            println!("PANIC! {} at {}", msg, "No location provided");
        }
        Some(loc) => {
            println!("PANIC! {} at {}:{}", msg, loc.file(), loc.line());
        }
    }
    loop {}
}

// Primary kernel bootstrap function.
// We ensure that we only initialize kernel subsystems
// one time by only doing so on hart0.
#[no_mangle]
pub extern "C" fn main() -> ! {
    // We only bootstrap on a single CPU.
    HAL::isolate();

    hal::HAL::global_setup();
    println!("{}", BANNER);
    log!(Info, "Bootstrapping on hart0...");
    match vm::global_init() {
        Ok(pt) => {
            unsafe {
                match KERNEL_PAGE_TABLE.set(pt) {
                    Ok(()) => {},
                    Err(_) => panic!("Kernel Page Table double init!"),
                }
                vm::local_init(KERNEL_PAGE_TABLE.get().unwrap());
            }
        },
        Err(_) => {
            panic!("Failed VM initialization!");
        }
    }
    log!(Info, "Initialized the kernel page table...");
    log!(Info, "Finished plic globl init...");
    unsafe {
        log!(Debug, "Testing page allocation and freeing...");
        vm::test_palloc();
        log!(Debug, "Testing galloc allocation and freeing...");
        vm::test_galloc();
    }
    log!(Debug, "Testing phys page extent allocation and freeing...");
    vm::test_phys_page();
    log!(Debug, "Successful phys page extent allocation and freeing...");

    // log!(Debug, "Initializing VIRTIO blk device...");
    // if let Err(e) = device::virtio::virtio_block_init() {
    //     println!("{:?}", e);
    // }
    // TODO rework virtio with opensbi. Discover how that should even work

    process::init_process_structure();
    log!(Debug, "Successfuly initialized the process system...");
    log!(Info, "Completed all hart0 initialization and testing...");

    unsafe {
        // release the waiting harts
        GLOBAL_INIT_FLAG.assume_init_mut().update(1);
        log!(Error, "Do CPU discovery and setup with HAL::wake_one");
    }

    // we want to test multiple processes with multiple harts
    process::test_multiprocess_syscall();

    panic!("Reached the end of kernel main! Did the root process not start?");
}
