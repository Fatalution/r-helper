use anyhow::Result;
use std::sync::mpsc;
use std::thread;
use crate::utils::{execute_powershell_command, clean_display_string};

#[derive(Debug, Clone)]
pub struct SystemSpecs {
    pub device_model: String,
    pub cpu_model: String,
    pub gpu_models: Vec<String>,
    pub total_ram_gb: u32,
}

impl Default for SystemSpecs {
    fn default() -> Self {
        Self {
            device_model: "Unknown".to_string(),
            cpu_model: "Unknown".to_string(),
            gpu_models: vec!["Unknown".to_string()],
            total_ram_gb: 0,
        }
    }
}

pub fn get_system_specs(device_name: Option<&str>) -> SystemSpecs {
    let mut specs = SystemSpecs::default();
    
    // Set device model from Razer device info if available
    if let Some(device) = device_name {
        // Truncate only the GPU model number from the end, keep the year
        // Handle formats like:
        // 'Razer Blade 16" (2025) 5090' -> 'Razer Blade 16" (2025)'
        // 'Razer Blade 15 (2024) 4070' -> 'Razer Blade 15 (2024)'
        let mut truncated_name = device;
        
        // Remove common GPU model patterns from the end (numbers like 4060, 4070, 5090, etc.)
        let gpu_patterns = ["4060", "4070", "4080", "4090", "5070", "5080", "5090"];
        for pattern in &gpu_patterns {
            // Look for the pattern at the end of the string (possibly with trailing whitespace)
            let pattern_with_space = format!(" {}", pattern);
            if truncated_name.ends_with(pattern) {
                truncated_name = &truncated_name[..truncated_name.len() - pattern.len()];
                break;
            } else if let Some(pos) = truncated_name.rfind(&pattern_with_space) {
                // Make sure this is actually at the end (after trimming whitespace)
                let after_pattern = &truncated_name[pos + pattern_with_space.len()..];
                if after_pattern.trim().is_empty() {
                    truncated_name = &truncated_name[..pos];
                    break;
                }
            }
        }
        
        specs.device_model = truncated_name.trim().to_string();
    }
    
    // Fetch CPU, GPU, RAM in parallel to reduce wall-clock time
    let (cpu_tx, cpu_rx) = mpsc::channel();
    let (gpu_tx, gpu_rx) = mpsc::channel();
    let (ram_tx, ram_rx) = mpsc::channel();

    thread::spawn(move || { let _ = cpu_tx.send(get_cpu_info()); });
    thread::spawn(move || { let _ = gpu_tx.send(get_gpu_info()); });
    thread::spawn(move || { let _ = ram_tx.send(get_ram_info()); });

    if let Ok(Ok(cpu)) = cpu_rx.recv() {
        specs.cpu_model = cpu;
    }
    if let Ok(Ok(gpus)) = gpu_rx.recv() {
        if !gpus.is_empty() { specs.gpu_models = gpus; }
    }
    if let Ok(Ok(ram_gb)) = ram_rx.recv() {
        specs.total_ram_gb = ram_gb;
    }
    
    specs
}

#[cfg(target_os = "windows")]
fn get_cpu_info() -> Result<String> {
    let script = "Get-WmiObject -Class Win32_Processor | Select-Object -ExpandProperty Name";
    let cpu_name = execute_powershell_command(script)?;
    Ok(clean_display_string(&cpu_name))
}

#[cfg(target_os = "windows")]
fn get_gpu_info() -> Result<Vec<String>> {
    let script = "Get-WmiObject -Class Win32_VideoController | Where-Object { $_.Name -notlike '*Virtual*' -and $_.Name -notlike '*Basic*' } | Select-Object -ExpandProperty Name";
    let output = execute_powershell_command(script)?;
    
    let gpu_names: Vec<String> = output
        .lines()
        .map(|line| clean_display_string(line))
        .filter(|line| !line.is_empty())
        .collect();
    
    if gpu_names.is_empty() {
        Ok(vec!["No discrete GPU detected".to_string()])
    } else {
        Ok(gpu_names)
    }
}

#[cfg(target_os = "windows")]
fn get_ram_info() -> Result<u32> {
    let script = "Get-WmiObject -Class Win32_ComputerSystem | Select-Object -ExpandProperty TotalPhysicalMemory";
    let output = execute_powershell_command(script)?;
    
    let ram_bytes_str = clean_display_string(&output);
    if let Ok(ram_bytes) = ram_bytes_str.parse::<u64>() {
        let ram_gb = (ram_bytes / (1024 * 1024 * 1024)) as u32;
        Ok(ram_gb)
    } else {
        Err(anyhow::anyhow!("Failed to parse RAM size"))
    }
}

#[cfg(not(target_os = "windows"))]
fn get_cpu_info() -> Result<String> {
    Err(anyhow::anyhow!("System specs detection only supported on Windows"))
}

#[cfg(not(target_os = "windows"))]
fn get_gpu_info() -> Result<Vec<String>> {
    Err(anyhow::anyhow!("System specs detection only supported on Windows"))
}

#[cfg(not(target_os = "windows"))]
fn get_ram_info() -> Result<u32> {
    Err(anyhow::anyhow!("System specs detection only supported on Windows"))
}
