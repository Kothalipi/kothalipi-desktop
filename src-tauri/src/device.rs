//! Kothalipi: what this computer can run, for model recommendations.
//!
//! Reads total memory, CPU cores and chip name, then sorts the machine into a
//! tier. The frontend maps tiers to models:
//! - `light`  (< 8 GB):  small models; Cloud mode suggested for best accuracy
//! - `standard` (8–15 GB): the recommended Bangla and English models
//! - `high`   (≥ 16 GB, or Apple Silicon with ≥ 12 GB): same models, with room
//!   to keep both loaded

use serde::Serialize;
use specta::Type;

#[derive(Serialize, Type, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceTier {
    Light,
    Standard,
    High,
}

#[derive(Serialize, Type, Debug, Clone)]
pub struct DeviceProfile {
    pub os: String,
    pub arch: String,
    /// e.g. "Apple M2" or "Intel(R) Core(TM) i5-8250U"; empty if unknown.
    pub chip: String,
    pub cpu_cores: u32,
    /// Total memory in whole gigabytes; 0 if unknown.
    pub ram_gb: u32,
    pub apple_silicon: bool,
    pub tier: DeviceTier,
}

pub fn tier_for(ram_gb: u32, apple_silicon: bool) -> DeviceTier {
    match ram_gb {
        0 => DeviceTier::Standard, // unknown: don't scare anyone off
        r if r >= 16 || (apple_silicon && r >= 12) => DeviceTier::High,
        r if r >= 8 => DeviceTier::Standard,
        _ => DeviceTier::Light,
    }
}

fn round_gb(bytes: u64) -> u32 {
    ((bytes as f64) / 1_073_741_824.0).round() as u32
}

#[cfg(target_os = "macos")]
fn sysctl_string(name: &str) -> Option<String> {
    let out = std::process::Command::new("sysctl")
        .args(["-n", name])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg(target_os = "macos")]
fn memory_bytes() -> Option<u64> {
    sysctl_string("hw.memsize")?.parse().ok()
}

#[cfg(target_os = "macos")]
fn chip_name() -> String {
    sysctl_string("machdep.cpu.brand_string").unwrap_or_default()
}

#[cfg(target_os = "linux")]
fn memory_bytes() -> Option<u64> {
    let info = std::fs::read_to_string("/proc/meminfo").ok()?;
    let kb: u64 = info
        .lines()
        .find(|l| l.starts_with("MemTotal:"))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()?;
    Some(kb * 1024)
}

#[cfg(target_os = "linux")]
fn chip_name() -> String {
    std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|info| {
            info.lines()
                .find(|l| l.starts_with("model name"))
                .and_then(|l| l.split(':').nth(1))
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_default()
}

#[cfg(target_os = "windows")]
fn memory_bytes() -> Option<u64> {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe { GlobalMemoryStatusEx(&mut status) }.ok()?;
    Some(status.ullTotalPhys)
}

#[cfg(target_os = "windows")]
fn chip_name() -> String {
    winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE)
        .open_subkey(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0")
        .and_then(|k| k.get_value::<String, _>("ProcessorNameString"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn memory_bytes() -> Option<u64> {
    None
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn chip_name() -> String {
    String::new()
}

pub fn profile() -> DeviceProfile {
    let ram_gb = memory_bytes().map(round_gb).unwrap_or(0);
    let apple_silicon = cfg!(all(target_os = "macos", target_arch = "aarch64"));
    DeviceProfile {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        chip: chip_name(),
        cpu_cores: std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(0),
        ram_gb,
        apple_silicon,
        tier: tier_for(ram_gb, apple_silicon),
    }
}

/// This computer's memory, cores and tier, for model recommendations.
#[tauri::command]
#[specta::specta]
pub fn get_device_profile() -> DeviceProfile {
    profile()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_follow_memory() {
        assert_eq!(tier_for(4, false), DeviceTier::Light);
        assert_eq!(tier_for(8, false), DeviceTier::Standard);
        assert_eq!(tier_for(12, false), DeviceTier::Standard);
        assert_eq!(tier_for(12, true), DeviceTier::High);
        assert_eq!(tier_for(32, false), DeviceTier::High);
        assert_eq!(tier_for(0, false), DeviceTier::Standard);
    }

    #[test]
    fn reads_this_machine() {
        let p = profile();
        assert!(p.cpu_cores > 0);
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        assert!(p.ram_gb > 0, "memory should be readable");
    }

    #[test]
    fn rounds_bytes_to_gb() {
        assert_eq!(round_gb(17_179_869_184), 16);
        assert_eq!(round_gb(8_000_000_000), 7);
    }
}
