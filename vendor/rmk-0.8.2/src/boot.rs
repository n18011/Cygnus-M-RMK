pub fn jump_to_bootloader() {
    #[cfg(feature = "adafruit_bl")]
    // Reference: https://github.com/adafruit/Adafruit_nRF52_Bootloader/blob/d6b28e66053eea467166f44875e3c7ec741cb471/src/main.c#L107
    embassy_nrf::pac::POWER
        .gpregret()
        .write_value(embassy_nrf::pac::power::regs::Gpregret(0x57));

    #[cfg(feature = "rp2040")]
    // Jump to RP2040 bootloader
    embassy_rp::rom_data::reset_to_usb_boot(0, 0);

    #[cfg(not(any(feature = "adafruit_bl", feature = "rp2040")))]
    warn!("Please specified a bootloader to jump to!");

    reboot_keyboard();
}

/// embassy-boot 0.7 state word at the standard rmk-boot page.
/// Only `SWAP_MAGIC` is rewritten, and only by clearing bits (`0xF0` → `0xD0`),
/// so a page erase is not required. Any other value is left alone.
pub async fn confirm_embassy_boot_swap<F>(flash: &mut F)
where
    F: embedded_storage_async::nor_flash::NorFlash,
{
    const STATE_ADDR: u32 = 0x0000_6000;
    const SWAP_MAGIC: [u8; 4] = [0xF0, 0xF0, 0xF0, 0xF0];
    const BOOT_MAGIC: [u8; 4] = [0xD0, 0xD0, 0xD0, 0xD0];

    let mut word = [0u8; 4];
    if flash.read(STATE_ADDR, &mut word).await.is_err() || word != SWAP_MAGIC {
        return;
    }
    // Failure leaves SWAP_MAGIC in place, so the bootloader can still roll back.
    let _ = flash.write(STATE_ADDR, &BOOT_MAGIC).await;
}

pub(crate) fn reboot_keyboard() {
    warn!("Rebooting keyboard!");
    // For cortex-m:
    #[cfg(all(
        target_arch = "arm",
        target_os = "none",
        any(target_abi = "eabi", target_abi = "eabihf")
    ))]
    cortex_m::peripheral::SCB::sys_reset();

    #[cfg(feature = "_esp_ble")]
    esp_hal::system::software_reset();
}
