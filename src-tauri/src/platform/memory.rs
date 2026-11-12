//! Process memory and machine resources: trimming after model eviction,
//! resident memory for the logs, physical cores and total RAM.

/// Return freed memory to the OS: mimalloc first, then the platform
/// allocator or working set.
pub fn trim() {
    // SAFETY: mi_collect only walks mimalloc's own heaps.
    unsafe { libmimalloc_sys::mi_collect(true) };
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    // SAFETY: malloc_trim(0) releases free glibc heap pages; no pointers.
    unsafe {
        libc::malloc_trim(0);
    }
    #[cfg(windows)]
    {
        use windows::Win32::System::ProcessStatus::EmptyWorkingSet;
        use windows::Win32::System::Threading::GetCurrentProcess;
        // SAFETY: GetCurrentProcess returns a pseudo handle that needs no
        // closing; EmptyWorkingSet only trims this process's working set.
        let _ = unsafe { EmptyWorkingSet(GetCurrentProcess()) };
    }
}

/// Resident memory of this process in bytes, if the OS reports it.
pub fn resident_bytes() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let line = status.lines().find(|l| l.starts_with("VmRSS:"))?;
        let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
        Some(kb * 1024)
    }
    #[cfg(windows)]
    {
        use windows::Win32::System::ProcessStatus::{
            GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
        };
        use windows::Win32::System::Threading::GetCurrentProcess;
        let mut c = PROCESS_MEMORY_COUNTERS::default();
        let cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        // SAFETY: `c` is a valid, correctly sized out-parameter.
        unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut c, cb) }.ok()?;
        Some(c.WorkingSetSize as u64)
    }
    #[cfg(target_os = "macos")]
    {
        let mut info: libc::proc_taskinfo = unsafe { std::mem::zeroed() };
        let size = std::mem::size_of::<libc::proc_taskinfo>() as libc::c_int;
        // SAFETY: proc_pidinfo writes at most `size` bytes into `info`.
        let n = unsafe {
            libc::proc_pidinfo(
                std::process::id() as libc::c_int,
                libc::PROC_PIDTASKINFO,
                0,
                (&mut info as *mut libc::proc_taskinfo).cast(),
                size,
            )
        };
        (n == size).then_some(info.pti_resident_size)
    }
    #[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
    {
        None
    }
}

/// Resident memory in MB for log lines.
pub fn resident_mb() -> String {
    resident_bytes().map_or_else(
        || "n/a".into(),
        |b| format!("{:.1}", b as f64 / 1_048_576.0),
    )
}

fn logical_cores() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get())
}

/// Number of physical CPU cores, falling back to logical cores.
pub fn physical_cores() -> usize {
    #[cfg(target_os = "linux")]
    {
        let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
        let mut cores = std::collections::HashSet::new();
        let mut phys = "";
        for line in cpuinfo.lines() {
            if let Some((k, v)) = line.split_once(':') {
                match k.trim() {
                    "physical id" => phys = v.trim(),
                    "core id" => {
                        cores.insert((phys.to_string(), v.trim().to_string()));
                    }
                    _ => {}
                }
            }
        }
        if cores.is_empty() {
            logical_cores()
        } else {
            cores.len()
        }
    }
    #[cfg(target_os = "macos")]
    {
        sysctl_u64("hw.physicalcpu").map_or_else(logical_cores, |n| n as usize)
    }
    #[cfg(windows)]
    {
        windows_physical_cores().unwrap_or_else(logical_cores)
    }
    #[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
    {
        logical_cores()
    }
}

/// Recogniser threads: `min(4, physical cores)` (plan P1.12).
pub fn stt_threads() -> i32 {
    physical_cores().clamp(1, 4) as i32
}

/// Total installed RAM in bytes.
pub fn total_ram_bytes() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;
        let line = meminfo.lines().find(|l| l.starts_with("MemTotal:"))?;
        let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
        Some(kb * 1024)
    }
    #[cfg(target_os = "macos")]
    {
        sysctl_u64("hw.memsize")
    }
    #[cfg(windows)]
    {
        use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        let mut m = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };
        // SAFETY: `m` is a valid out-parameter with dwLength set.
        unsafe { GlobalMemoryStatusEx(&mut m) }.ok()?;
        Some(m.ullTotalPhys)
    }
    #[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
    {
        None
    }
}

#[cfg(target_os = "macos")]
fn sysctl_u64(name: &str) -> Option<u64> {
    let cname = std::ffi::CString::new(name).ok()?;
    let mut buf = [0u8; 8];
    let mut len: libc::size_t = buf.len();
    // SAFETY: sysctlbyname writes at most `len` bytes into `buf`.
    let rc = unsafe {
        libc::sysctlbyname(
            cname.as_ptr(),
            buf.as_mut_ptr().cast(),
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    match (rc, len) {
        (0, 4) => Some(u64::from(u32::from_ne_bytes([
            buf[0], buf[1], buf[2], buf[3],
        ]))),
        (0, 8) => Some(u64::from_ne_bytes(buf)),
        _ => None,
    }
}

#[cfg(windows)]
fn windows_physical_cores() -> Option<usize> {
    use windows::Win32::System::SystemInformation::{
        GetLogicalProcessorInformation, RelationProcessorCore, SYSTEM_LOGICAL_PROCESSOR_INFORMATION,
    };
    let mut len = 0u32;
    // SAFETY: the first call only reports the needed buffer length.
    let _ = unsafe { GetLogicalProcessorInformation(None, &mut len) };
    let item = std::mem::size_of::<SYSTEM_LOGICAL_PROCESSOR_INFORMATION>();
    if len == 0 || item == 0 {
        return None;
    }
    let mut buf = vec![SYSTEM_LOGICAL_PROCESSOR_INFORMATION::default(); len as usize / item + 1];
    // SAFETY: `buf` holds at least `len` bytes.
    unsafe { GetLogicalProcessorInformation(Some(buf.as_mut_ptr()), &mut len) }.ok()?;
    let n = (len as usize / item).min(buf.len());
    let cores = buf[..n]
        .iter()
        .filter(|i| i.Relationship == RelationProcessorCore)
        .count();
    (cores > 0).then_some(cores)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resources_are_reported() {
        assert!(physical_cores() >= 1);
        assert!((1..=4).contains(&stt_threads()));
        assert!(total_ram_bytes().unwrap_or(0) > 256 * 1024 * 1024);
        assert!(resident_bytes().unwrap_or(0) > 0);
        trim();
    }
}
