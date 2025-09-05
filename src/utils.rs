// Utility functions shared across the app

use std::process::Command;
use anyhow::Result;

pub use anyhow;

// System Command Execution

/// Execute a PowerShell script with consistent configuration
/// 
/// This function provides a centralized way to execute PowerShell commands
/// with proper error handling and consistent flags.
#[cfg(target_os = "windows")]
pub fn execute_powershell_command(script: &str) -> Result<String> {
    use std::os::windows::process::CommandExt;
    
    let mut cmd = Command::new(POWERSHELL_PATH);
    cmd.args(&["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command"])
       .arg(script)
       .creation_flags(CREATE_NO_WINDOW);
    
    match cmd.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr_string = String::from_utf8_lossy(&output.stderr);
            let stderr = stderr_string.trim();
            
            if !stderr.is_empty() && output.status.code() != Some(0) {
                Err(anyhow::anyhow!("PowerShell error: {}", stderr))
            } else {
                Ok(stdout)
            }
        },
        Err(e) => Err(anyhow::anyhow!("Failed to execute PowerShell: {}", e))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn execute_powershell_command(_script: &str) -> Result<String> {
    Err(anyhow::anyhow!("PowerShell is only available on Windows"))
}

// String Processing Utilities

/// Clean and format strings for display
pub fn clean_display_string(input: &str) -> String {
    input
        .trim()
        .replace('\r', "")
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// Device Command Utilities

/// Helper function for device command execution with standard error handling
/// Returns Ok(success_message) or Err(error_message)
pub fn execute_device_command_simple<T, F>(
    device_opt: Option<&librazer::device::Device>,
    command: F,
    success_msg: &str,
    error_prefix: &str,
) -> Result<String, String>
where
    F: FnOnce(&librazer::device::Device) -> Result<T>,
{
    if let Some(device) = device_opt {
        match command(device) {
            Ok(_) => Ok(success_msg.to_string()),
            Err(e) => Err(format!("{}: {}", error_prefix, e)),
        }
    } else {
        Err("No device connected".to_string())
    }
}

// Device State Management

/// Batch read multiple device states with error handling
pub struct DeviceStateReader<'a> {
    device: &'a librazer::device::Device,
    errors: Vec<String>,
}

impl<'a> DeviceStateReader<'a> {
    pub fn new(device: &'a librazer::device::Device) -> Self {
        Self {
            device,
            errors: Vec::new(),
        }
    }
    
    pub fn read<T, F>(&mut self, operation: F, operation_name: &str) -> Option<T>
    where
        F: FnOnce(&librazer::device::Device) -> Result<T>,
    {
        match operation(self.device) {
            Ok(value) => Some(value),
            Err(e) => {
                self.errors.push(format!("Failed to read {}: {}", operation_name, e));
                None
            }
        }
    }
    
    pub fn finish(self) -> Vec<String> {
        self.errors
    }
}

// Constants

/// PowerShell executable path on Windows
#[cfg(target_os = "windows")]
pub const POWERSHELL_PATH: &str = "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe";

/// Windows creation flag to hide console window
#[cfg(target_os = "windows")]
pub const CREATE_NO_WINDOW: u32 = 0x08000000;

