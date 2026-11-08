//! Whole-process-tree resource measurement.
//!
//! A Tauri app on Windows is not one process: WebView2 spawns a browser
//! process plus renderer/GPU/utility children. Reporting only the main exe
//! would understate memory by a large factor, so DiZero always measures the
//! full tree rooted at its own PID.
//!
//! Metrics (per process, summed for the tree):
//! * **Working set** — physical memory currently mapped (what Task Manager's
//!   "Memory" column approximates). Includes shared pages, so the sum across
//!   processes over-counts shared DLLs slightly.
//! * **Private bytes** — committed memory private to the process (commit charge).
//! * **CPU %** — kernel+user time delta between samples, normalised by the
//!   number of logical CPUs (100% = every core busy).

use serde::Serialize;
use std::collections::HashMap;
use std::time::Instant;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProcessMetrics {
    pub pid: u32,
    pub parent_pid: u32,
    pub name: String,
    #[ts(type = "number")]
    pub working_set_bytes: u64,
    #[ts(type = "number")]
    pub private_bytes: u64,
    /// Private working set: resident memory not shared with other processes
    /// (Task Manager's "Memory" column). `None` on Windows builds that don't
    /// report it.
    #[ts(type = "number | null")]
    pub private_working_set_bytes: Option<u64>,
    /// `None` on the first sample (no delta yet).
    pub cpu_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PerfSnapshot {
    pub supported: bool,
    pub processes: Vec<ProcessMetrics>,
    #[ts(type = "number")]
    pub total_working_set_bytes: u64,
    #[ts(type = "number")]
    pub total_private_bytes: u64,
    /// Sum of private working sets; `None` if any process didn't report one.
    #[ts(type = "number | null")]
    pub total_private_working_set_bytes: Option<u64>,
    pub total_cpu_percent: Option<f64>,
    pub logical_cpus: u32,
    #[ts(type = "number")]
    pub sampled_at: i64,
}

/// Raw per-process reading.
#[derive(Debug, Clone)]
pub struct RawProcess {
    pub pid: u32,
    pub parent_pid: u32,
    pub name: String,
    pub working_set: u64,
    pub private_bytes: u64,
    pub private_working_set: Option<u64>,
    /// Kernel + user time in 100 ns units.
    pub cpu_time: u64,
}

/// Every process whose ancestry leads to `root` (including `root`).
/// `all` is `(pid, parent_pid)`. Cycle-safe.
pub fn descendants(all: &[(u32, u32)], root: u32) -> Vec<u32> {
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    for &(pid, parent) in all {
        if pid != parent {
            children.entry(parent).or_default().push(pid);
        }
    }
    let mut out = vec![root];
    let mut seen = std::collections::HashSet::from([root]);
    let mut i = 0;
    while i < out.len() {
        if let Some(kids) = children.get(&out[i]) {
            for &k in kids {
                if seen.insert(k) {
                    out.push(k);
                }
            }
        }
        i += 1;
    }
    out
}

pub struct PerfSampler {
    root_pid: u32,
    last: Option<(Instant, HashMap<u32, u64>)>,
    logical_cpus: u32,
}

impl PerfSampler {
    pub fn new(root_pid: u32) -> Self {
        let logical_cpus = std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(1);
        PerfSampler { root_pid, last: None, logical_cpus }
    }

    pub fn sample(&mut self) -> PerfSnapshot {
        let now = Instant::now();
        let Some(raw) = platform::read_tree(self.root_pid) else {
            return PerfSnapshot {
                supported: false,
                processes: vec![],
                total_working_set_bytes: 0,
                total_private_bytes: 0,
                total_private_working_set_bytes: None,
                total_cpu_percent: None,
                logical_cpus: self.logical_cpus,
                sampled_at: crate::util::now_ms(),
            };
        };
        let prev = self.last.take();
        let elapsed_100ns = prev.as_ref().map(|(t, _)| now.duration_since(*t).as_nanos() as f64 / 100.0);
        let denom = elapsed_100ns.map(|e| e * self.logical_cpus as f64).filter(|d| *d > 0.0);
        let mut cpu_now = HashMap::with_capacity(raw.len());
        let mut total_cpu = denom.map(|_| 0.0);
        let processes: Vec<ProcessMetrics> = raw
            .into_iter()
            .map(|p| {
                cpu_now.insert(p.pid, p.cpu_time);
                let cpu_percent = match (&prev, denom) {
                    (Some((_, last)), Some(d)) => {
                        // New processes since the last sample count from zero.
                        let before = last.get(&p.pid).copied().unwrap_or(0);
                        let pct = (p.cpu_time.saturating_sub(before) as f64 / d * 100.0).clamp(0.0, 100.0);
                        if let Some(t) = total_cpu.as_mut() {
                            *t += pct;
                        }
                        Some(pct)
                    }
                    _ => None,
                };
                ProcessMetrics {
                    pid: p.pid,
                    parent_pid: p.parent_pid,
                    name: p.name,
                    working_set_bytes: p.working_set,
                    private_bytes: p.private_bytes,
                    private_working_set_bytes: p.private_working_set,
                    cpu_percent,
                }
            })
            .collect();
        self.last = Some((now, cpu_now));
        PerfSnapshot {
            supported: true,
            total_working_set_bytes: processes.iter().map(|p| p.working_set_bytes).sum(),
            total_private_bytes: processes.iter().map(|p| p.private_bytes).sum(),
            total_private_working_set_bytes: processes.iter().map(|p| p.private_working_set_bytes).sum(),
            total_cpu_percent: total_cpu.map(|t: f64| t.min(100.0)),
            processes,
            logical_cpus: self.logical_cpus,
            sampled_at: crate::util::now_ms(),
        }
    }
}

#[cfg(windows)]
mod platform {
    use super::{RawProcess, descendants};
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX2,
    };
    use windows_sys::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
    };

    fn ft(f: &FILETIME) -> u64 {
        ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64
    }

    fn snapshot() -> Option<Vec<(u32, u32, String)>> {
        // SAFETY: Toolhelp snapshot API used per its documented contract; the
        // handle is closed on every path and the entry struct is sized first.
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap == INVALID_HANDLE_VALUE {
                return None;
            }
            let mut out = Vec::new();
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            if Process32FirstW(snap, &mut entry) != 0 {
                loop {
                    let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                    let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
                    out.push((entry.th32ProcessID, entry.th32ParentProcessID, name));
                    if Process32NextW(snap, &mut entry) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snap);
            Some(out)
        }
    }

    fn read_process(pid: u32) -> Option<(u64, u64, Option<u64>, u64)> {
        // SAFETY: OpenProcess/GetProcessMemoryInfo/GetProcessTimes with
        // correctly sized out-structs; handle closed before returning.
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ, 0, pid);
            if h.is_null() {
                return None;
            }
            // EX2 adds PrivateWorkingSetSize; older Windows builds leave it 0.
            let mut mem: PROCESS_MEMORY_COUNTERS_EX2 = std::mem::zeroed();
            mem.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX2>() as u32;
            let mem_ok = GetProcessMemoryInfo(h, &mut mem as *mut _ as *mut PROCESS_MEMORY_COUNTERS, mem.cb) != 0;
            let (mut c, mut e, mut k, mut u): (FILETIME, FILETIME, FILETIME, FILETIME) =
                (std::mem::zeroed(), std::mem::zeroed(), std::mem::zeroed(), std::mem::zeroed());
            let times_ok = GetProcessTimes(h, &mut c, &mut e, &mut k, &mut u) != 0;
            CloseHandle(h);
            if !mem_ok {
                return None;
            }
            let cpu = if times_ok { ft(&k) + ft(&u) } else { 0 };
            let pws = (mem.PrivateWorkingSetSize > 0).then_some(mem.PrivateWorkingSetSize as u64);
            Some((mem.WorkingSetSize as u64, mem.PrivateUsage as u64, pws, cpu))
        }
    }

    pub fn read_tree(root: u32) -> Option<Vec<RawProcess>> {
        let all = snapshot()?;
        let pairs: Vec<(u32, u32)> = all.iter().map(|(p, pp, _)| (*p, *pp)).collect();
        let tree = descendants(&pairs, root);
        let mut out = Vec::with_capacity(tree.len());
        for pid in tree {
            let Some((_, parent, name)) = all.iter().find(|(p, _, _)| *p == pid) else { continue };
            if let Some((ws, private, pws, cpu)) = read_process(pid) {
                out.push(RawProcess {
                    pid,
                    parent_pid: *parent,
                    name: name.clone(),
                    working_set: ws,
                    private_bytes: private,
                    private_working_set: pws,
                    cpu_time: cpu,
                });
            }
        }
        Some(out)
    }
}

#[cfg(not(windows))]
mod platform {
    use super::RawProcess;
    /// Only Windows is implemented in this build.
    pub fn read_tree(_root: u32) -> Option<Vec<RawProcess>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descendants_walks_the_tree_and_ignores_unrelated() {
        // 1 → 2 → 3, 1 → 4, 9 unrelated (parent 8)
        let all = [(1, 0), (2, 1), (3, 2), (4, 1), (9, 8)];
        let mut d = descendants(&all, 1);
        d.sort();
        assert_eq!(d, vec![1, 2, 3, 4]);
    }

    #[test]
    fn descendants_survives_parent_cycles() {
        let all = [(1, 2), (2, 1)];
        let d = descendants(&all, 1);
        assert_eq!(d.len(), 2);
    }

    #[cfg(windows)]
    #[test]
    fn samples_the_current_process() {
        let mut s = PerfSampler::new(std::process::id());
        let first = s.sample();
        assert!(first.supported);
        assert!(first.processes.iter().any(|p| p.pid == std::process::id()));
        assert!(first.total_working_set_bytes > 1024 * 1024);
        assert!(first.total_private_bytes > 0);
        // Private working set (what Task Manager's Memory column shows) is
        // part of the shared set, so it can never exceed the working set.
        let me = first.processes.iter().find(|p| p.pid == std::process::id()).unwrap();
        let pws = me.private_working_set_bytes.expect("private working set is available on Windows 10 20348+/11");
        assert!(pws > 0 && pws <= me.working_set_bytes);
        assert_eq!(
            first.total_private_working_set_bytes,
            Some(first.processes.iter().filter_map(|p| p.private_working_set_bytes).sum())
        );
        assert!(first.total_cpu_percent.is_none(), "first sample has no CPU delta");
        let mut x = 0u64;
        for i in 0..2_000_000u64 {
            x = x.wrapping_add(i * i);
        }
        std::hint::black_box(x);
        let second = s.sample();
        assert!(second.total_cpu_percent.is_some());
    }
}
