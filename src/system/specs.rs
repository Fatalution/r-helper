use anyhow::Result;
use crate::utils::{execute_powershell_command, clean_display_string};

#[derive(Debug, Clone)]
pub struct SystemSpecs {
    pub device_model: String,
    pub gpu_models: Vec<String>,
}

impl Default for SystemSpecs {
    fn default() -> Self {
        Self {
            device_model: "Unknown".to_string(),
            gpu_models: vec!["Unknown".to_string()],
        }
    }
}

pub fn get_system_specs(device_name: Option<&str>) -> SystemSpecs {
    let mut specs = SystemSpecs::default();
    
    // Set device model from Razer device info if available
    if let Some(device) = device_name {
        // Keep only: model + inch size + optional year (e.g., "Razer Blade 16" (2025)")
        specs.device_model = simplify_model_name(device);
    }
    
    // Fetch GPU info (Windows only); ignore CPU and RAM
    if let Ok(gpus) = get_gpu_info() {
        if !gpus.is_empty() { specs.gpu_models = gpus; }
    }
    
    specs
}

// Short and robust: keep up to the year if present; otherwise keep up to the inch size after "Blade".
fn simplify_model_name(name: &str) -> String {
    let s = name.trim();
    // Prefer: everything up to closing paren of the year
    if let Some(open) = s.find('(') {
        if let Some(close_rel) = s[open..].find(')') {
            return s[..open + close_rel + 1].trim().to_string();
        }
    }
    // Fallback: keep up to the first size number (digits) after "Blade", incl. optional '"'
    if let Some(blade_pos) = s.find("Blade") {
        if let Some(rel_digit) = s[blade_pos..].find(|c: char| c.is_ascii_digit()) {
            let mut end = blade_pos + rel_digit;
            let bytes = s.as_bytes();
            while end < s.len() && bytes[end].is_ascii_digit() { end += 1; }
            if end < s.len() && bytes[end] == b'"' { end += 1; }
            return s[..end].trim().to_string();
        }
    }
    s.to_string()
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

#[cfg(not(target_os = "windows"))]
fn get_gpu_info() -> Result<Vec<String>> {
    Err(anyhow::anyhow!("System specs detection only supported on Windows"))
}
