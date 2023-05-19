### The current assuptions for context switching rely on these
### functions leaving the interrupt stack (sscratch stack) *exactly*
### as it was found, and not pushing or popping anything that remains
### / disappears after the trap exits


        .section .text
        ## This is the machine mode trap vector(not really). It exists
        ## to get us into the rust handler
        .option norvc
        .align 4
        .global __mtrapvec
__mtrapvec:
        csrrw sp, mscratch, sp
        save_gp_regs

        .extern m_handler
        call m_handler

        load_gp_regs
        csrrw sp, mscratch, sp
        mret

### ------------------------------------------------------------------
###
### Start of S mode stuff

        ## This is the supervisor trap handler
        .option norvc
        .align 4
        .globl __strapvec
__strapvec:
        csrrw sp, sscratch, sp
        sd t0, -8(sp)
        ## do early direction
        csrr t0, scause
        addi t0, t0, -8
        bnez t0, regular_strap
        ## Single out u mode scall
        ##
        ## I want to handle that seperately, reset state and move to
        ## handler
        ld t0, -8(sp)
        csrrw sp, sscratch, sp
        ## back to program stack
        j scall_asm

### handling a trap that was not a U mode syscall
###
### This is on the interrupt stack
regular_strap:
        ld t0, -8(sp)
        save_gp_regs

        ## load kernel page table
        ld t1, 264(sp)          #256 + 8

        li a0, 1
        sll a0, a0, 63
        ## top bit
        srl t1, t1, 12
        or t1, t1, a0
        ## top bit mode and PPN

        sfence.vma x0, x0
        csrrw s1, satp, t1
        sfence.vma x0, x0
        ## now in kernel space, note that s1 should not be distrubed
        ## by rust

        ## get gp back to restore more info from later
        ld gp, 256(sp)

        .extern s_handler
        call s_handler

        sfence.vma x0, x0
        csrw satp, s1
        sfence.vma x0, x0

        load_gp_regs
        csrrw sp, sscratch, sp
        sret


        ## The ecall / syscall handler is here.
        ##
        ## It follows the linux riscv calling convention for syscalls
        ##
        ## See
        ## https://stackoverflow.com/questions/59800430/risc-v-ecall-syscall-calling-convention-on-pk-linux
        ##
        ## This expects the call number in a7
        ## the arguments in a0-a5
        ## return value in a0
        ##
        ## The convention is that the caller saved registers are free
        ## to clobber as with a regular call
        ##
        ## The convention leaves a6 unused but safe to clobber, so we
        ## will use it for other communication purposes, specifically
        ## directing traffic
        ##
scall_asm:
        ## handle a yield specifically

        ## make quick space by using the sscratch stack without
        ## changing its value
        csrrw sp, sscratch, sp
        addi sp, sp, -64
        sd a0, (sp)
        sd a1, 8(sp)
        sd a2, 16(sp)
        sd a3, 24(sp)
        sd a4, 32(sp)
        sd a5, 40(sp)
        sd a6, 48(sp)
        sd a7, 56(sp)
        ## we are on the sscratch stack and can clobber a0 freely. All
        ## others must be preserved
        jal scall_direct

        ## returns zero in a0 if we want to stay on the program
        ## stack/page table, and non-zero for the kernel stack/ page
        ## table

        beqz a0, dont_change_stack
### -=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=
### This is the context switch
###
### there is code reuse from proc_space_to_kernel_space, but because
### of stack changes, this cannot be made into a asm function for
### reuse

        ## a regs are caller saved, we can clobber here
        addi sp, sp, 64

        ## change stacks/page table here
        ld a0, (sp)
        addi sp, sp, 8
        csrrw sp, sscratch, sp
        ## now program register state is as it was when scall was
        ## executed, and we are back on the program stack
        save_gp_regs
        ## onto PROCESS stack

        ## hold onto what we need to save
        csrr s2, sepc
        mv s3, sp
        ## These two must be preserved across several calls until they
        ## might be used in scall_rust

        ## sscratch holds the interrupt stack
        csrr sp, sscratch

        ## sscratch stack holds, from low addr to high:
        ##
        ## the addr to restore to gp (see hartlocal.rs)
        ## the kernel page table (satp)
        ## the kernel stack (sp)

        ## load kernel page table
        ld t1, 8(sp)

        li t0, 1
        sll t0, t0, 63
        ## top bit
        srl t1, t1, 12
        or t1, t1, t0
        ## top bit mode and PPN

        sfence.vma x0, x0
        csrw satp, t1
        sfence.vma x0, x0

        ## get gp back to restore more info from later
        ld gp, (sp)
        ## get on the main kernel stack
        ld sp, 16(sp)

        j enter_rust_caller
### -=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=
### This is the end of the context switch
### We are fully in kernel space now.
### The program pc is in a0 and the program sp is in a1

dont_change_stack:
        ld a0, (sp)
        ld a1, 8(sp)
        ld a2, 16(sp)
        ld a3, 24(sp)
        ld a4, 32(sp)
        ld a5, 40(sp)
        ld a6, 48(sp)
        ld a7, 56(sp)
        addi sp, sp, 64

        csrrw sp, sscratch, sp  #program stack

### We are on the appropriate stack, whatever it is, and we can get
### into the rust caller to do our work. Note that if you entered in
### kernel space/stack, you need to exit back to the process with a
### process_resume instead of just returning from rust handler
enter_rust_caller:
        addi sp, sp, -8
        sd ra, (sp)
        ## call the main handler
        jal scall_rust
        ## we only actually come out of this for non-kernel directed
        ## syscalls, so we don't need to worry about more stack stuff
        ## here

        ## this is odd in terms of name, but is already saved for us,
        ## so might as well use it
        csrr ra, sepc
        addi ra, ra, 4
        csrw sepc, ra

        ld ra, (sp)
        addi sp, sp, 8


        sret
