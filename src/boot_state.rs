//! Confirm an embassy-boot swap without erasing flash.
//!
//! rmk-boot's standard (and non-defmt `noswap`) state page starts at `0x6000`.
//! After a swap-mode update the first word is `0xF0F0F0F0` (`SWAP_MAGIC`).
//! Changing it to `0xD0D0D0D0` (`BOOT_MAGIC`) only clears bits, so NVMC can
//! program it without a page erase. Any other value is left untouched, so a
//! different bootloader image at that address is not modified.
//!
//! The acknowledgement runs from `__pre_init`, before RAM init and before
//! MPSL owns NVMC. A `defmt` bootloader keeps its state page elsewhere; this
//! code will not see that word.
//!
//! NVMC base `0x4001E000`, READY at `+0x400`, CONFIG at `+0x504` (nRF52840).
//! CONFIG.WEN = 1 enables write, 0 returns to read.

/// Word to program over `current`, if it is a pending swap marker.
#[cfg(test)]
pub fn boot_word_if_swap_pending(current: u32) -> Option<u32> {
    const SWAP_MAGIC: u32 = 0xF0F0_F0F0;
    const BOOT_MAGIC: u32 = 0xD0D0_D0D0;
    if current == SWAP_MAGIC { Some(BOOT_MAGIC) } else { None }
}

#[cfg(not(test))]
core::arch::global_asm!(
    r#"
.global __pre_init
.type __pre_init,%function
.thumb_func
__pre_init:
    ldr r0, =0x00006000
    ldr r1, [r0]
    ldr r2, =0xF0F0F0F0
    cmp r1, r2
    bne 2f
    ldr r3, =0x4001E000
0:
    ldr r1, [r3, #0x400]
    tst r1, #1
    beq 0b
    movs r1, #1
    str r1, [r3, #0x504]
    ldr r1, =0xD0D0D0D0
    str r1, [r0]
1:
    ldr r2, [r3, #0x400]
    tst r2, #1
    beq 1b
    movs r1, #0
    str r1, [r3, #0x504]
2:
    bx lr
    "#
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_swap_magic_is_confirmed() {
        assert_eq!(boot_word_if_swap_pending(0xF0F0_F0F0), Some(0xD0D0_D0D0));
        assert_eq!(boot_word_if_swap_pending(0xD0D0_D0D0), None);
        assert_eq!(boot_word_if_swap_pending(0xFFFF_FFFF), None);
        assert_eq!(boot_word_if_swap_pending(0xC0C0_C0C0), None);
        assert_eq!(boot_word_if_swap_pending(0xE0E0_E0E0), None);
        assert_eq!(boot_word_if_swap_pending(0xF0F0_F0F1), None);
    }
}
