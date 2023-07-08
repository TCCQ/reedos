### This file contains the entry code for the S mode entry from uboot
### + opensbi. It replaces entry.s and _start, as those are roughly
### handled by uboot / opensbi. The main thing we want to do is setup
### the stacks / sscratch stacks

        .option norvc
        .section .text.entry
        .global _entry
_entry:
        mv a3, a0
        li a0, 0x3000           #2 page stack + guard page
        mul a1, a3, a0          #offset by hart id
        .extern _stacks_end
        la a2, _stacks_end      # this is the top byte for hart 0
        sub sp, a2, a1

        ## We want to do a similar thing for the interupt stacks

        ## we can't reuse the offset (a1) because the spacing is different
        li a0, 0x2000           #2 page stack + guard page
        mul a1, a3, a0          #offset by hart id
        .extern _intstacks_end
        la a2, _intstacks_end
        sub a2, a2, a1
        ## int stack base in a2 now.

        ## put half of the initial contents of the sscratch stack in
        ## now, namely the kernel stack base addr for this hart
        addi a2, a2, -8
        sd sp, (a2)
        csrw sscratch, a2 # Write per hart sscratch pad

        .extern main
        j main
