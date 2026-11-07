/*
 * Candidate linker script for the standard non-defmt rmk-boot nRF52840 layout.
 *
 * This matches rmk-boot v0.9.0 `rmk-nrf52840-memory.x` and current
 * rmk-boot `build.rs` (24 KiB bootloader, 4 KiB state, ACTIVE at 0x7000).
 * The same ACTIVE origin is used by the non-defmt `noswap` build.
 * It does NOT match a `defmt` bootloader (32 KiB, ACTIVE moves) or `dfu_ext`.
 *
 * USB IDs do not prove which of those builds is installed. Do not treat this
 * file as a measurement of the board.
 *
 * https://github.com/rmk-rs/rmk-boot/releases/download/v0.9.0/rmk-nrf52840-memory.x
 */
MEMORY {
  FLASH : ORIGIN = 0x00007000, LENGTH = 491520   /* ACTIVE region */
  RAM   : ORIGIN = 0x20000000, LENGTH = 256K
}

__bootloader_state_start   = 0x6000;
__bootloader_state_end     = 0x7000;
__bootloader_active_start  = 0x7000;
__bootloader_active_end    = 0x7F000;
__bootloader_dfu_start     = 0x7F000;
__bootloader_dfu_end       = 0xF8000;
__bootloader_storage_start = 0xF8000;
__bootloader_storage_end   = 0x100000;
