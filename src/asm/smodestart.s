### This file contains the entry code for the S mode entry from uboot
### + opensbi. It replaces entry.s and _start, as those are roughly
### handled by uboot / opensbi. The main thing we want to do is setup
### the stacks / sscratch stacks

        .option norvc
        .section .text.entry
        .global _entry
_entry:
        csrr a1, mhartid
        li a0, 0x3000           #2 page stack + guard page
        mul a1, a1, a0          #offset by hart id
        .extern _stacks_end
        la a2, _stacks_end      # this is the top byte for hart 0
        sub sp, a2, a1

        ## .extern _intstacks_end
        ## csrr a1, mhartid
        ## li a0, 0x4000
        ## mul a1, a1, a0
        ## la a2, _intstacks_end
        ## sub a2, a2, a1
        ## csrw mscratch, a2 # Write per hart mscratch pad

        ## ^ commented because we don't have to handle m mode ints anymore


        li a0, 0x2000
        sub a2, a2, a0 # Move sp down by scratch pad page + guard page

        ## put half of the initial contents of the sscratch stack in
        ## now, namely the kernel stack base addr for this hart
        addi a2, a2, -8
        sd sp, (a2)
        csrw sscratch, a2 # Write per hart sscratch pad

        .extern main
        j main
