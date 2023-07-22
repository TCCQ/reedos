//! This module should contain all the stuff for wrapping the wasm
//! execution evironment that makes up our safe extension support.


// -------------------------------------------------------------------
// This supplies the stuff that wasm-m-r expects to link against.

// This project should have a top level subdir that is a git submodule
// for our wasm runtime found here:
// https://github.com/bytecodealliance/wasm-micro-runtime

/****************************************************
 *                     Section 1                    *
 *        Interfaces required by the runtime        *
 ****************************************************/

/**
 * Initialize the platform internal resources if needed,
 * this function is called by wasm_runtime_init() and
 * wasm_runtime_full_init()
 *
 * @return 0 if success
 */
// int
// bh_platform_init(void);
#[no_mangle]
pub extern "C" fn bh_platform_init() -> i32 {
    todo!("wasm init");
    0
}

/**
 * Destroy the platform internal resources if needed,
 * this function is called by wasm_runtime_destroy()
 */
// void
// bh_platform_destroy(void);
#[no_mangle]
pub extern "C" fn bh_platform_destroy() {
    todo!("wasm destruction");
}

/**
 ******** memory allocator APIs **********
 */

// void *
// os_malloc(unsigned size);
#[no_mangle]
pub extern "C" fn os_malloc(size: u32) -> *mut u8 {
    todo!("wasm malloc");
    0 as *mut u8
}


// void *
// os_realloc(void *ptr, unsigned size);
#[no_mangle]
pub extern "C" fn os_realloc(ptr: *mut u8, size: u32) -> *mut u8 {
    todo!("wasm realloc");
    0 as *mut u8
}

// void
// os_free(void *ptr);
#[no_mangle]
pub extern "C" fn os_free(ptr: *mut u8) {
    todo!("wasm free");
}

/**
 * Note: the above APIs can simply return NULL if wasm runtime
 *       isn't initialized with Alloc_With_System_Allocator.
 *       Refer to wasm_runtime_full_init().
 */

// int
// os_printf(const char *format, ...);
#[no_mangle]
pub extern "C" fn os_printf(format: *const u8, ...) {
    todo!("wasm printf")
}

// int
// os_vprintf(const char *format, va_list ap);

// TODO rust extern C variatics
#[no_mangle]
pub extern "C" fn os_vprintf(format: *const u8, ...) {
    todo!("wasm vprintf")
}


/**
 * Get microseconds after boot.
 */
// uint64
// os_time_get_boot_microsecond(void);
#[no_mangle]
pub extern "C" fn os_time_get_book_miscrosecond() -> u64 {
    todo!("wasm get-time");
    0
}


/**
 * Get current thread id.
 * Implementation optional: Used by runtime for logging only.
 */
// korp_tid
// os_self_thread(void);


/**
 * Get current thread's stack boundary address, used for runtime
 * to check the native stack overflow. Return NULL if it is not
 * easy to implement, but may have potential issue.
 */
// uint8 *
// os_thread_get_stack_boundary(void);
#[no_mangle]
pub extern "C" fn os_thread_get_stack_boundary() -> *const u8 {
    todo!("wasm stack boundary");
    0 as *const u8
}


/**
 ************** mutext APIs ***********
 *  vmcore:  Not required until pthread is supported by runtime
 *  app-mgr: Must be implemented
 */

// int
// os_mutex_init(korp_mutex *mutex);

// int
// os_mutex_destroy(korp_mutex *mutex);

// int
// os_mutex_lock(korp_mutex *mutex);

// int
// os_mutex_unlock(korp_mutex *mutex);

/**************************************************
 *                    Section 2                   *
 *            APIs required by WAMR AOT           *
 **************************************************/

/* Memory map modes */
#[no_mangle]
pub extern "C" {
    const MMAP_PROT_NONE: u32 = 0;
    const MMAP_PROT_READ: u32  = 1;
    const MMAP_PROT_WRITE: u32 = 2;
    const MMAP_PROT_EXEC: u32 = 4;
}

/* Memory map flags */
#[no_mangle]
pub extern "C" {
    const MMAP_MAP_NONE: u32 = 0;
    /* Put the mapping into 0 to 2 G; supported only on x86_64 */
    const MMAP_MAP_32BIT: u32 = 1;
    /* Don't interpret addr as a hint: place the mapping at exactly
       that address. */
    const MMAP_MAP_FIXED: u32 = 2;
}

// void *
// os_mmap(void *hint, size_t size, int prot, int flags);
// void
// os_munmap(void *addr, size_t size);
// int
// os_mprotect(void *addr, size_t size, int prot);
#[no_mangle]
pub extern "C" fn os_mmap(hint: *mut u8, size: usize, prot: u32, flags: u32) -> *mut u8 {
    todo!("wasm mmap");
    0 as *mut u8
}

#[no_mangle]
pub extern "C" fn os_munmap(addr: *mut u8, size: usize) -> *mut u8 {
    todo!("wasm munmap");
    0 as *mut u8
}

#[no_mangle]
pub extern "C" fn os_mprotect(addr: *mut u8, size: usize, prot: u32) -> *mut u8 {
    todo!("wasm mprotect");
    0 as *mut u8
}

/**
 * Flush cpu data cache, in some CPUs, after applying relocation to the
 * AOT code, the code may haven't been written back to the cpu data cache,
 * which may cause unexpected behaviour when executing the AOT code.
 * Implement this function if required, or just leave it empty.
 */
// void
// os_dcache_flush(void);
#[no_mangle]
pub extern "C" fn os_dcache_flush() {
    todo!("wasm cache flush");
}

