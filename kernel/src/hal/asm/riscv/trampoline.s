        ## This file contains the asm for context switches, the last
        ## thing that is run in kernel mode on a switch in, and the
        ## first thing on a switch out

.macro save_gp_regs
    addi sp, sp, -256

    sd x0, 0(sp)
    sd x1, 8(sp)
    sd x2, 16(sp)
    sd x3, 24(sp)
    sd x4, 32(sp)
    sd x5, 40(sp)
    sd x6, 48(sp)
    sd x7, 56(sp)
    sd x8, 64(sp)
    sd x9, 72(sp)
    sd x10, 80(sp)
    sd x11, 88(sp)
    sd x12, 96(sp)
    sd x13, 104(sp)
    sd x14, 112(sp)
    sd x15, 120(sp)
    sd x16, 128(sp)
    sd x17, 136(sp)
    sd x18, 144(sp)
    sd x19, 152(sp)
    sd x20, 160(sp)
    sd x21, 168(sp)
    sd x22, 176(sp)
    sd x23, 184(sp)
    sd x24, 192(sp)
    sd x25, 200(sp)
    sd x26, 208(sp)
    sd x27, 216(sp)
    sd x28, 224(sp)
    sd x29, 232(sp)
    sd x30, 240(sp)
.endm

.macro load_gp_regs
    ld x0, 0(sp)
    ld x1, 8(sp)
    ld x2, 16(sp)
    ld x3, 24(sp)
    ld x5, 40(sp)
    ld x6, 48(sp)
    ld x7, 56(sp)
    ld x8, 64(sp)
    ld x9, 72(sp)
    ld x10, 80(sp)
    ld x11, 88(sp)
    ld x12, 96(sp)
    ld x13, 104(sp)
    ld x14, 112(sp)
    ld x15, 120(sp)
    ld x16, 128(sp)
    ld x17, 136(sp)
    ld x18, 144(sp)
    ld x19, 152(sp)
    ld x20, 160(sp)
    ld x21, 168(sp)
    ld x22, 176(sp)
    ld x23, 184(sp)
    ld x24, 192(sp)
    ld x25, 200(sp)
    ld x26, 208(sp)
    ld x27, 216(sp)
    ld x28, 224(sp)
    ld x29, 232(sp)
    ld x30, 240(sp)

    addi sp, sp, 256
.endm

        ## jump into a process that hasn't been run yet
        ## pc in a0, new base pagetable addr in a1, sp in a2
        ##
        ## we don't need to worry about saving registers, as this is a
        ## non-returning function call
        .global process_start_asm
process_start_asm:
        csrw sepc, a0
        ## return to the process on sret

        ## before swapping anything, we need to save the gp back to
        ## the top of the sscratch stack
        csrr a0, sscratch
        sd gp, (a0)

        mv sp, a2
        ## get onto the process stack, we will restore kernel stack
        ## with sscratch later

        li a0, 1
        sll a0, a0, 63
        ## top bit
        srl a1, a1, 12
        or a1, a1, a0
        ## top bit mode and PPN

        sfence.vma x0, x0
        csrw satp, a1
        sfence.vma x0, x0
        ## swap tables

        sret
        ## enter the process with usermode and pc/satp
        ##
        ## TODO do I need to worry about prior priviledge level not
        ## being U?

### ------------------------------------------------------------------

        ## jump into a process that has been run before
        ## takes pc in a0, new base pt in a1, and new sp in a2
        .global process_resume_asm
process_resume_asm:
        csrw sepc, a0
        ## return to the process on sret


        ## before we swap page tables, we need to save the gp info to
        ## a place we can restore to later
        ##
        ## Specifically the top of the sscratch stack
        csrr a0, sscratch
        sd gp, (a0)

        li a0, 1
        sll a0, a0, 63
        ## top bit
        srl a1, a1, 12
        or a1, a1, a0
        ## top bit mode and PPN

        sfence.vma x0, x0
        csrw satp, a1
        sfence.vma x0, x0
        ## swap tables

        mv sp, a2
        load_gp_regs
        sret
        ## jump there and enter U mode
        ## TODO worry about prior priv != U mode?

### ------------------------------------------------------------------
        ## this is the end of the file
