/* nRF52840: 1 MiB of flash at 0, 256 KiB of RAM at 0x2000_0000. The bridge
   has no statics (its buffers are on the stack of a function that never
   returns), so there is no data or bss to initialize. */
MEMORY
{
  FLASH : ORIGIN = 0x00000000, LENGTH = 1024K
  RAM   : ORIGIN = 0x20000000, LENGTH = 256K
}
ENTRY(reset)
SECTIONS
{
  .vectors ORIGIN(FLASH) :
  {
    LONG(ORIGIN(RAM) + LENGTH(RAM));
    KEEP(*(.vectors));
  } > FLASH
  .text :
  {
    *(.text .text.*);
    *(.rodata .rodata.*);
  } > FLASH
  /DISCARD/ : { *(.ARM.exidx .ARM.exidx.* .ARM.extab .ARM.extab.*); }
}
