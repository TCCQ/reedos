OUTPUT_ARCH( "riscv" )
ENTRY( _entry )

/*
 * Including this breaks the .= ? before text. It is just ignored.
 *
 * MEMORY
 * {
 *   RAM  (wxa) : ORIGIN = 0x80000000, LENGTH = 2048M
 * }
 */

SECTIONS
{
  PROVIDE(_memory_start = 0x80000000);
  . = 0xA0000000;
  .text : {
    PROVIDE(_text_start = .);
    *(.text.entry)
    *(.text .text.*)
    . = ALIGN(0x1000);
    PROVIDE(_text_end = .);
  }

  /* should be < page, with alignment EXACTLY a page */
  /*
   * .trampoline : {
   *   . = ALIGN(0x1000);
   *   PROVIDE(_trampoline_start = .);
   *   *(.trampoline)8000
   *   *(.trampoline.*)
   *   . = ALIGN(0x1000);
   *   PROVIDE(_trampoline_end = .);
   * }
   */

  /*
   * Only required when using a dynamically linked ELF, which we are not
   * PROVIDE(_global_pointer = .);8000
   */

  .rodata : {
    PROVIDE(_rodata_start = .);
    *(.srodata .srodata.*)
    *(.rodata .rodata.*)
    . = ALIGN(0x1000);
    PROVIDE(_rodata_end = .);
  }
  .data : {
    PROVIDE(_data_start = .);
    *(.sdata .sdata.*)
    *(.data .data.*)
    . = ALIGN(0x1000);
    PROVIDE(_data_end = .);
  }

  /* lower guard page included in above */
  .stacks : {
    . = ALIGN(0x1000);
    PROVIDE(_stacks_start = .);
    . = . + (4096 * 9 * 2); /* NHARTS with a guard page each, unstable */
    /* This is wild. *EACH* kernel stack gets 8 full pages because wasmi eats stack for breakfast */
    PROVIDE(_stacks_end = .);
  }
  .intstacks : {
    . = ALIGN(0x1000);
    PROVIDE(_intstacks_start = .);
    . = . + (0x1000 * 4 * 2);
    PROVIDE(_intstacks_end = .);
  }
  . = . + 4096; /* guard page */
  /* stacks should start at stack end and alternate with guard pages going down */

  .bss : {
    . = ALIGN(0x1000);
    PROVIDE(_bss_start = .);
    *(.sbss .sbss.*)
    *(.bss .bss.*)
    . = ALIGN(0x1000);
    PROVIDE(_bss_end = .);
  }

  /*
   * uneeded?
   * PROVIDE(_end = .);
   */
  /*
   * PROVIDE(_trampoline_target = ORIGIN(RAM) + LENGTH(RAM) - 0x1000);
   */
  /*
   * PROVIDE(_trampoline_target = ORIGIN(RAM) + LENGTH(RAM) - 0x1000);
   */
  /*
   * PROVIDE(_memory_end = ORIGIN(RAM) + LENGTH(RAM));
   */
  PROVIDE(_memory_end = 0xA0000000 + 256M);
}
