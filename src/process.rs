//! Process handle and utilities.
// use alloc::boxed::Box;

// extern crate alloc;

// use alloc::boxed::Box;
use alloc::collections::vec_deque::*;
use core::assert;
use core::mem::{size_of, MaybeUninit};
use core::ptr::copy_nonoverlapping;
use core::cell::OnceCell;

use crate::hal::*;
// use crate::hw::HartContext;
// use crate::trap::TrapFrame;
// use crate::vm::ptable::*;
use crate::vm::VmError;
// use crate::hw::riscv::read_tp;
// use crate::hw::param::*;
use crate::vm::{request_phys_page, PhysPageExtent};
use crate::file::elf64::*;
use crate::hw::hartlocal::*;
use crate::lock::mutex::Mutex;
use crate::id::IdGenerator;


mod scheduler;
use crate::process::scheduler::ProcessQueue;


static mut PID_COUNTER: Mutex<IdGenerator> = Mutex::new(IdGenerator::new());

#[allow(unused_variables)]
mod syscall;
// This should not be exposed to anything, and we don't need to call
// any of it here

// for now we wil be using a single locked round robin queue
static mut QUEUE: OnceCell<Mutex<ProcessQueue>> = OnceCell::new();


/// Global init for all process related stuff. Not exaustive, also
/// need hartlocal_info_interrupt_stack_init
pub fn init_process_structure() {
    unsafe {
        match QUEUE.set(Mutex::new(ProcessQueue::new())) {
            Ok(()) => {},
            Err(_) => {
                panic!("Process structure double init!");
            },
        }
    }
}

// use hart local info to get the currently running process
//
// this is a *MOVE* of the process. Handle elsewhere
fn get_running_process() -> Process {
    restore_gp_info64().current_process
}

#[derive(Debug)]
pub enum ProcessState {
    Uninitialized,              // do not attempt to run
    Unstarted,                  // do not attempt to restore regs
    Ready,                      // can run, restore args
    Running,                    // is running, don't use elsewhere
    // ^ is because ownership alone is risky to ensure safety accross
    // context switches
    Wait,                       // blocked on on something
    Sleep,                      // out of the running for a bit
    Dead,                       // do not run (needed?)
}

/// A process. The there is a real possiblity of this being largly
/// uninitialized, so check the state always
pub struct Process {
    saved_pc: usize,            // uninit with 0
    saved_sp: usize,            // uninit with 0
    id: usize,                  // uninit with 0
    state: ProcessState,        // use uninit state
    pgtbl: PageTable,                     // uninizalied with null
    phys_pages: MaybeUninit<VecDeque<PhysPageExtent>>, // vec to avoid Ord requirement
    // ^ hopefully it's clear how this is uninit
    // TODO consider this as a OnceCell

    // sleep_time: usize           // uninit with 0, only valid with sleep state

    // currently unused, but needed in the future
    // address_space: BTreeSet<Box<dyn Resource>>, // todo: Balanced BST of Resources

}

// TODO merge this with ELFError?
#[derive(Debug)]
pub enum ProcError {
    OOM,
}

fn user_process_flags(r: bool, w: bool, e: bool) -> PageMapFlags {
    PageMapFlags::User |
    if r {PageMapFlags::Read} else {PageMapFlags::empty()} |
    if w {PageMapFlags::Write} else {PageMapFlags::empty()} |
    if e {PageMapFlags::Execute} else {PageMapFlags::empty()}
}

fn kernel_process_flags(r: bool, w: bool, e: bool) -> PageMapFlags {
    PageMapFlags::empty() |
    if r {PageMapFlags::Read} else {PageMapFlags::empty()} |
    if w {PageMapFlags::Write} else {PageMapFlags::empty()} |
    if e {PageMapFlags::Execute} else {PageMapFlags::empty()}
}

impl Process {
    /// Construct a new process. Notably does not allocate anything or
    /// mean anything until you initialize it.
    pub fn new_uninit() -> Result<Self, ProcError> {
        let out = Self {
            id: 0,
            state: ProcessState::Uninitialized,
            pgtbl: match HAL::pgtbl_new_empty() {
                Ok(p) => p,
                Err(_) => return Err(ProcError::OOM),
            },
            phys_pages: MaybeUninit::uninit(),
            saved_pc: 0,
            saved_sp: 0,
        };
        Ok(out)
    }

    pub fn initialize64(&mut self, elf: &ELFProgram) -> Result<(), ELFError> {
        // Doesn't assert uninitialized state so you can do a write over of an existing process

        match self.state {
            ProcessState::Uninitialized => {
                self.id = unsafe {PID_COUNTER.lock().generate()};
                self.pgtbl = match HAL::pgtbl_new_empty() {
                    Ok(p) => p,
                    Err(_) => return Err(ELFError::FailedAlloc),
                };
                self.phys_pages.write(VecDeque::new());
                // phys_pages DOES NOT include the pagetable. The
                // pagetable is floating memory, and MUST be cleaned
                // up with HAL::pgtbl_free
            },
            ProcessState::Running => {
                panic!("Tried to re-initialize a running process!");
            },
            _ => {},
        }

        self.populate_pagetable64(elf)?;
        match self.map_kernel_text() {
            Ok(_) => {},
            Err(_) => {
                panic!("Failed to map kernel text into process space!");
            }
        }
        self.saved_pc = elf.header.entry;
        self.state = ProcessState::Unstarted;
        Ok(())
    }

    // TODO is this the right error type?
    fn map_kernel_text(&mut self) -> Result<(), VmError> {
        // This is currently a large copy of kpage_init with a few tweaks

        // same closure trick as the main kernel mapping to collect errors
        let map_kernel = || {
            HAL::pgtbl_insert_range(
                self.pgtbl,
                HAL::text_start() as VirtAddress,
                HAL::text_start() as PhysAddress,
                HAL::text_end().addr() - HAL::text_start().addr(),
                kernel_process_flags(true, false, true)
            )?;

            HAL::pgtbl_insert_range(
                self.pgtbl,
                HAL::text_end(),
                HAL::text_end() as *mut usize,
                HAL::rodata_end().addr() - HAL::text_end().addr(),
                kernel_process_flags(true, false, false),
            )?;

            HAL::pgtbl_insert_range(
                self.pgtbl,
                HAL::rodata_end(),
                HAL::rodata_end() as *mut usize,
                HAL::data_end().addr() - HAL::rodata_end().addr(),
                kernel_process_flags(true, true, false),
            )?;

            // This maps hart 0, 1 stack pages in opposite order as entry.S. Shouln't necessarily be a
            // problem.
            let base = HAL::stacks_start();
            for s in 0..HAL::NHART {
                let stack = unsafe { base.byte_add(PAGE_SIZE * (1 + s * 3)) };
                HAL::pgtbl_insert_range(
                    self.pgtbl,
                    stack,
                    stack,
                    PAGE_SIZE * 2,
                    kernel_process_flags(true, true, false),
                )?;
            }

            // This maps hart 0, 1 stack pages in opposite order as entry.S. Shouln't necessarily be a
            // problem.
            let base = HAL::intstacks_start();
            for i in 0..HAL::NHART {
                let m_intstack = unsafe { base.byte_add(PAGE_SIZE * (1 + i * 4)) };
                // Map hart i m-mode handler.
                HAL::pgtbl_insert_range(
                    self.pgtbl,
                    m_intstack,
                    m_intstack,
                    PAGE_SIZE,
                    kernel_process_flags(true, true, false),
                )?;
                // Map hart i s-mode handler
                let s_intstack = unsafe { m_intstack.byte_add(PAGE_SIZE * 2) };
                HAL::pgtbl_insert_range(
                    self.pgtbl,
                    s_intstack,
                    s_intstack,
                    PAGE_SIZE,
                    kernel_process_flags(true, true, false),
                )?;
            }

            HAL::pgtbl_insert_range(
                self.pgtbl,
                HAL::bss_start(),
                HAL::bss_start(),
                HAL::bss_end().addr() - HAL::bss_start().addr(),
                kernel_process_flags(true, true, false),
            )?;

            HAL::pgtbl_insert_range(
                self.pgtbl,
                HAL::bss_end(),
                HAL::bss_end(),
                HAL::memory_end().addr() - HAL::bss_end().addr(),
                kernel_process_flags(true, true, false),
            )?;
            Ok::<(), HALVMError>(())
        };

        match map_kernel() {
            Ok(()) => Ok(()),
            Err(_) => Err(VmError::Koom), // TODO our error handling/typing/naming is totally unclear
        }
    }


    // TODO better error type here?
    /// Copies the LOAD segment memory layout from the elf to the
    /// program. This is not the only initialization step.
    ///
    /// This also setups up the program stack and sets saved_sp
    fn populate_pagetable64(&mut self, elf: &ELFProgram) -> Result<(), ELFError>{
        assert!(elf.header.program_entry_size as usize == size_of::<ProgramHeaderSegment64>(),
                "Varying ELF entry size expectations.");

        let num = elf.header.num_program_entries;
        let ptr = unsafe {
            elf.source.add(elf.header.program_header_pos)
                as *const ProgramHeaderSegment64
        };
        for i in 0..num {
            let segment = unsafe { *ptr.add(i as usize) };
            if segment.seg_type != ProgramSegmentType::Load { continue; }
            else if segment.vmem_addr < 0x1000  { return Err(ELFError::MappedZeroPage) }
            else if segment.vmem_addr >= HAL::text_start().addr() as u64 &&
                segment.vmem_addr <= HAL::text_end().addr() as u64 {
                    return Err(ELFError::MappedKernelText)
                }
            else if segment.size_in_file != segment.size_in_memory {return Err(ELFError::InequalSizes)}
            else if segment.alignment > 0x1000 {return Err(ELFError::ExcessiveAlignment)}

            let n_pages = (segment.size_in_memory + (0x1000 - 1)) / 0x1000;
            let pages = match request_phys_page(n_pages as usize) {
                Ok(p) => {p},
                Err(_) => {return Err(ELFError::FailedAlloc)}
            };
            unsafe {
                copy_nonoverlapping(elf.source.add(segment.file_offset as usize),
                                    pages.start() as *mut u8,
                                    segment.size_in_file as usize);
            }
            let flags = user_process_flags(
                (segment.flags as u16) & PROG_SEG_READ != 0,
                (segment.flags as u16) & PROG_SEG_WRITE != 0,
                (segment.flags as u16) & PROG_SEG_EXEC != 0
            );

            match HAL::pgtbl_insert_range(
                self.pgtbl,
                VirtAddress::from(segment.vmem_addr as *mut usize),
                PhysAddress::from(pages.start() as *mut usize),
                n_pages as usize,
                flags
            ) {
                Ok(_) => {},
                Err(_) => {return Err(ELFError::FailedMap)}
            }
            unsafe {
                self.phys_pages.assume_init_mut().push_back(pages);
            }
        }

        // TODO what does process heap look like? depends on our syscalls I guess?
        // We would map it here if we had any

        // map the process stack. They will get 2 pages for now
        const STACK_PAGES: usize = 2;
        let stack_pages = match request_phys_page(STACK_PAGES) {
            Ok(p) => {p},
            Err(_) => {
                return Err(ELFError::FailedAlloc);
            }
        };
        // TODO guard page? you'll get a page fault anyway?
        let process_stack_location = unsafe {
            HAL::text_start().sub(0x1000 * STACK_PAGES)
        };
        // under the kernel text
        match HAL::pgtbl_insert_range(
            self.pgtbl,
            VirtAddress::from(process_stack_location),
            PhysAddress::from(stack_pages.start()),
            STACK_PAGES,
            user_process_flags(true, true, false)
        ) {
            Ok(_) =>{},
            Err(_) => {return Err(ELFError::FailedMap)}
        }
        self.saved_sp = stack_pages.end() as usize;
        unsafe {
            self.phys_pages.assume_init_mut().push_back(stack_pages);
        }

        Ok(())
    }

    /// This is a (kind of) context switch
    ///
    /// This consumes the process from the rust perspective, but it is
    /// actually preserved elsewhere (gp info) and restored. This is
    /// because we need to preserve info across entering and exiting
    /// the process, but no non-global rust location does that, and we
    /// can't use a global array or anything like htat because we need
    /// to have each hart's process's lifetime be independent, and
    /// further, it doesn't make sense to have Process be Sync when it
    /// is not.
    ///
    /// TODO consider if there is a non-gp solution involing global
    /// pointers to heap allocated locations per hart. That is
    /// conceptually what is going on, but I still think we would have
    /// Sync/Send issues
    pub fn start(mut self) -> ! {
        match self.state {
            ProcessState::Unstarted => {},
            _ => {panic!("Attempted to start an already started program!")},
        }
        self.state = ProcessState::Running;

        extern "C" {pub fn process_start_asm(pc: usize, pgtbl: usize, sp: usize) -> !;}

        let saved_pc = self.saved_pc;
        let pgtbl_base = self.pgtbl.addr as usize;
        let saved_sp = self.saved_sp;
        let gpi = GPInfo::new(self);
        save_gp_info64(gpi);

        unsafe {
            // we can't use PageTable.write_satp here becuase this is
            // not mapped into the process pagetable and it shouldn't
            // be. We want to do that later in the asm.
            //
            // relies on args in a0, a1, a2 in order (see extern C)
            process_start_asm(saved_pc, pgtbl_base, saved_sp);
        }
    }

    /// This is our main context switch. Back into a running process
    /// from kernel space
    ///
    /// See above comment about data movement of a process struct
    pub fn resume(mut self) -> ! {
        match self.state {
            ProcessState::Ready => {},
            _ => {
                panic!("Attempted to resume a process that was not marked as Ready.")
            },
        }
        self.state = ProcessState::Running;

        extern "C" {pub fn process_resume_asm(pc: usize, pgtbl: usize, sp: usize) -> !;}

        let saved_pc = self.saved_pc;
        let pgtbl_base = self.pgtbl.addr as usize;
        let saved_sp = self.saved_sp;
        let gpi = GPInfo::new(self);
        save_gp_info64(gpi);

        unsafe {
            process_resume_asm(saved_pc, pgtbl_base, saved_sp);
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        match self.state {
            ProcessState::Running => {
                panic!("Tried to drop a running process!");
            }
            _ => {}
        }
        unsafe { PID_COUNTER.lock().free(self.id); }

        HAL::pgtbl_free(self.pgtbl);
        // dropping the phys pages vector will automatically clean
        // those up
    }
}

/// Suspend process so that it can be restored/restarted later. Called
/// from syscalls currently
fn process_pause(pc: usize, sp: usize, cause: usize) -> ! {
    let mut proc = get_running_process();
    proc.saved_pc = pc + 4;
    // ^ ecall doesn't automatically increment pc
    proc.saved_sp = sp;
    // TODO enum for causes?
    match cause {
        0 => {
            proc.state = ProcessState::Ready;
        },
        _ => {
            panic!("Unknown reason for process swap.");
        }
    }

    // log!(Debug, "Hart {}: Process {} yielded.", read_tp(), proc.id);


    // This is careful code to avoid holding the lock when we enter
    // the process, as that would lead to an infinite lock
    let next;
    unsafe {
        let mut locked = QUEUE.get().unwrap().lock();
        locked.insert(proc);
        next = locked.get_ready_process();
    }
    match next.state {
        ProcessState::Ready => {next.resume()},
        ProcessState::Unstarted => {next.start()},
        _ => {panic!("Bad process state from scheduler!")}
    }
}

#[no_mangle]
pub extern "C" fn process_exit_rust(exit_code: isize) -> ! {
    let proc = get_running_process();
    log!(Debug, "Process {} exited with code {}.", proc.id, exit_code);
    drop(proc);
    // ^ ensure that the never returning scheduler call doesn't extend
    // the life of the process


    // This is careful code to avoid holding the lock when we enter
    // the process, as that would lead to an infinite lock
    let next;
    unsafe {
        let mut locked = QUEUE.get().unwrap().lock();
        next = locked.get_ready_process();
    }
    match next.state {
        ProcessState::Ready => {next.resume()},
        ProcessState::Unstarted => {next.start()},
        _ => {panic!("Bad process state from scheduler!")}
    }
}


pub fn _test_process_spin() {
    let bytes = include_bytes!("programs/spin/spin.elf");
    let program = ELFProgram::new64(&bytes[0] as *const u8);
    let mut proc = Process::new_uninit().expect("Failed to create test process");

    match proc.initialize64(&program) {
        Ok(_) => {},
        Err(e) => {panic!("Couldn't start process: {:?}", e)}
    }
    proc.start();
}

pub fn _test_process_syscall_basic() {
    let bytes = include_bytes!("programs/syscall-basic/syscall-basic.elf");
    let program = ELFProgram::new64(&bytes[0] as *const u8);
    let mut proc = Process::new_uninit().expect("Failed to create test process");

    match proc.initialize64(&program) {
        Ok(_) => {},
        Err(e) => {panic!("Couldn't start process: {:?}", e)}
    }
    proc.start();
}

pub fn test_multiprocess_syscall() {
    let bytes = include_bytes!("programs/syscall-basic/syscall-basic.elf");
    let program = ELFProgram::new64(&bytes[0] as *const u8);
    let mut proc = Process::new_uninit().expect("Failed to create test process");

    match proc.initialize64(&program) {
        Ok(_) => {},
        Err(e) => {panic!("Couldn't start process: {:?}", e)}
    }

    for _ in 0..4 {
        let mut proc = Process::new_uninit().expect("Failed to create test process");

        match proc.initialize64(&program) {
            Ok(_) => {},
            Err(e) => {panic!("Couldn't start process: {:?}", e)}
        }

        unsafe {
            QUEUE.get().unwrap().lock().insert(proc)
        }
    }

    let enter;
    unsafe {
        enter = QUEUE.get().unwrap().lock().get_ready_process();
    }
    match enter.state {
        ProcessState::Unstarted => enter.start(),
        ProcessState::Ready => enter.resume(),
        _ => {panic!()}
    }

}

// TODO is there a better place for this stuff?
// /// Moving to `mod process`
// pub trait Resource {}

// /// Moving to `mod <TBD>`
// pub struct TaskList {
//     head: Option<Box<Process>>,
// }

// /// Moving to `mod <TBD>`
// pub struct TaskNode {
//     proc: Option<Box<Process>>,
//     prev: Option<Box<TaskNode>>,
//     next: Option<Box<TaskNode>>,
// }
