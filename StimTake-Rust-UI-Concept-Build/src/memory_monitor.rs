//! Windows system/process memory telemetry and stable pressure classification.
//! Monitoring failures are non-destructive: this module never changes overlay activation.
use serde::Serialize;
use std::{thread, time::{Duration, SystemTime, UNIX_EPOCH}};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryPressure { Green, Yellow, Red }

#[derive(Debug, Clone, Copy)]
pub struct MemorySnapshot {
    pub load_percent: u32,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub process_working_set_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryStatus {
    pub pressure: MemoryPressure,
    pub system_load: Option<u32>,
    pub total_physical_bytes: Option<u64>,
    pub available_physical_bytes: Option<u64>,
    pub process_working_set_bytes: Option<u64>,
    pub sampled_at_unix: Option<u64>,
    pub last_error: Option<String>,
    pub monitor_interval_seconds: u64,
}
impl Default for MemoryStatus {
    fn default() -> Self { Self { pressure: MemoryPressure::Green, system_load: None, total_physical_bytes: None, available_physical_bytes: None, process_working_set_bytes: None, sampled_at_unix: None, last_error: None, monitor_interval_seconds: 2 } }
}

#[cfg(windows)]
pub fn read_memory() -> Result<MemorySnapshot, String> {
    use std::mem::size_of;
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::Threading::GetCurrentProcess;

    let mut status = MEMORYSTATUSEX::default();
    status.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
    // SAFETY: status is a valid, initialized MEMORYSTATUSEX and dwLength is set per Win32 docs.
    unsafe { GlobalMemoryStatusEx(&mut status).map_err(|e| format!("GlobalMemoryStatusEx: {e}"))?; }

    let mut counters = PROCESS_MEMORY_COUNTERS::default();
    let counters_size = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    counters.cb = counters_size;
    // SAFETY: counters has the documented structure layout and cb size; GetCurrentProcess is a pseudo-handle.
    let process_working_set_bytes = unsafe {
        GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters_size)
            .ok().map(|_| counters.WorkingSetSize as u64)
    };

    Ok(MemorySnapshot {
        load_percent: status.dwMemoryLoad,
        total_bytes: status.ullTotalPhys,
        available_bytes: status.ullAvailPhys,
        process_working_set_bytes,
    })
}

#[cfg(not(windows))]
pub fn read_memory() -> Result<MemorySnapshot, String> {
    Err("Windows memory monitoring is only available on Windows.".into())
}

/// Hysteresis thresholds are StimTake policy defaults, not Windows thresholds.
pub fn classify(current: MemoryPressure, load: u32, available_bytes: u64) -> MemoryPressure {
    const GIB: u64 = 1024 * 1024 * 1024;
    match current {
        MemoryPressure::Green => {
            if load >= 85 || available_bytes < GIB { MemoryPressure::Red }
            else if load >= 75 || available_bytes < 2 * GIB { MemoryPressure::Yellow }
            else { MemoryPressure::Green }
        }
        MemoryPressure::Yellow => {
            if load >= 85 || available_bytes < GIB { MemoryPressure::Red }
            else if load <= 68 && available_bytes >= 2 * GIB { MemoryPressure::Green }
            else { MemoryPressure::Yellow }
        }
        MemoryPressure::Red => {
            if load <= 68 && available_bytes >= 2 * GIB { MemoryPressure::Green }
            else if load <= 78 && available_bytes >= GIB { MemoryPressure::Yellow }
            else { MemoryPressure::Red }
        }
    }
}

pub fn start(shared: Arc<Mutex<super::Shared>>) {
    thread::spawn(move || {
        let mut pressure = MemoryPressure::Green;
        let mut pending: Option<MemoryPressure> = None;
        let mut pending_count = 0u8;
        loop {
            match read_memory() {
                Ok(snapshot) => {
                    let desired = classify(pressure, snapshot.load_percent, snapshot.available_bytes);
                    if desired == pressure { pending = None; pending_count = 0; }
                    else if pending == Some(desired) { pending_count = pending_count.saturating_add(1); }
                    else { pending = Some(desired); pending_count = 1; }

                    // Three consecutive samples (6s) to change pressure, preventing flapping.
                    if pending_count >= 3 {
                        pressure = desired;
                        pending = None;
                        pending_count = 0;
                    }
                    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                    if let Ok(mut state) = shared.lock() {
                        state.memory = MemoryStatus {
                            pressure,
                            system_load: Some(snapshot.load_percent),
                            total_physical_bytes: Some(snapshot.total_bytes),
                            available_physical_bytes: Some(snapshot.available_bytes),
                            process_working_set_bytes: snapshot.process_working_set_bytes,
                            sampled_at_unix: Some(timestamp),
                            last_error: None,
                            monitor_interval_seconds: 2,
                        };
                    }
                }
                Err(error) => {
                    if let Ok(mut state) = shared.lock() {
                        state.memory.last_error = Some(error);
                        // Preserve last successful sample and pressure. Never change overlay activation.
                    }
                }
            }
            thread::sleep(Duration::from_secs(2));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    const GIB: u64 = 1024 * 1024 * 1024;
    #[test] fn green_enters_yellow_at_policy_threshold() { assert_eq!(classify(MemoryPressure::Green, 75, 4*GIB), MemoryPressure::Yellow); }
    #[test] fn green_enters_red_at_policy_threshold() { assert_eq!(classify(MemoryPressure::Green, 85, 4*GIB), MemoryPressure::Red); }
    #[test] fn yellow_requires_lower_exit_threshold() { assert_eq!(classify(MemoryPressure::Yellow, 70, 3*GIB), MemoryPressure::Yellow); assert_eq!(classify(MemoryPressure::Yellow, 68, 3*GIB), MemoryPressure::Green); }
    #[test] fn red_recovers_through_pressure_bands() { assert_eq!(classify(MemoryPressure::Red, 80, 3*GIB), MemoryPressure::Red); assert_eq!(classify(MemoryPressure::Red, 78, 3*GIB), MemoryPressure::Yellow); }
}
