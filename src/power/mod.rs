use anyhow::Result;

#[cfg(target_os = "windows")]
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

#[cfg(target_os = "windows")]
pub fn get_power_state() -> Result<bool> {
    unsafe {
        let mut status: SYSTEM_POWER_STATUS = std::mem::zeroed();
        if GetSystemPowerStatus(&mut status).is_ok() {
            Ok(status.ACLineStatus == 1)
        } else {
            Ok(true)
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn get_power_state() -> Result<bool> {
    Ok(true)
}
