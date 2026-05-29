use std::collections::HashMap;
use std::time::Instant;
use sysinfo::{Disks, Networks, ProcessRefreshKind, ProcessesToUpdate, System};

use crate::models::{DiskInfo, NetworkInterface, RamMetrics, TopProcess};

/// Handles repeated sysinfo polls for CPU, RAM, disks, and network.
pub struct Monitor {
    sys: System,
    disks: Disks,
    networks: Networks,
    /// Per-interface totals from the last poll: name → (sent_bytes, recv_bytes).
    prev_net: HashMap<String, (u64, u64)>,
    /// Wall-clock time of the last network poll, for accurate delta calculation.
    last_net_poll: Instant,
    /// Cached disk snapshot (refreshed at most every ~10 s).
    disk_cache: Vec<DiskInfo>,
    last_disk_refresh: Instant,
    /// Rolling buffer of the last 3 raw CPU readings.
    /// Averaging smooths out single-sample spikes from hardware driver queries.
    cpu_history: [f32; 3],
    cpu_history_idx: usize,
    /// Number of logical CPUs, used to normalise per-process CPU%.
    cpu_count: f32,
}

impl Monitor {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        // sysinfo needs two CPU snapshots separated by at least
        // MINIMUM_CPU_UPDATE_INTERVAL to produce accurate readings.
        // Without this warmup the first collect() can return garbage (100%).
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        sys.refresh_cpu_usage();

        let mut networks = Networks::new_with_refreshed_list();
        networks.refresh();
        // Seed per-interface totals so the first collect() produces a valid delta.
        let prev_net: HashMap<String, (u64, u64)> = networks
            .iter()
            .map(|(name, data)| {
                (name.clone(), (data.total_transmitted(), data.total_received()))
            })
            .collect();

        let mut disks = Disks::new_with_refreshed_list();
        disks.refresh();
        let disk_cache = collect_disks(&disks);

        let cpu_count = sys.cpus().len().max(1) as f32;

        // First process refresh — gives CPU baselines for the next delta.
        sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::new().with_cpu(),
        );

        Self {
            sys,
            disks,
            networks,
            prev_net,
            last_net_poll: Instant::now(),
            disk_cache,
            last_disk_refresh: Instant::now(),
            cpu_history: [0.0; 3],
            cpu_history_idx: 0,
            cpu_count,
        }
    }

    /// Refresh all sensors and return current snapshots.
    /// Disks are re-queried at most once every 10 seconds.
    pub fn collect(&mut self) -> (f32, RamMetrics, Vec<DiskInfo>, Vec<NetworkInterface>, Option<TopProcess>) {
        // CPU — rolling average over 3 samples to smooth hardware-driver spikes
        self.sys.refresh_cpu_usage();
        let raw_cpu = self.sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>()
            / self.sys.cpus().len() as f32;
        self.cpu_history[self.cpu_history_idx % 3] = raw_cpu;
        self.cpu_history_idx = self.cpu_history_idx.wrapping_add(1);
        // Until the buffer is full (first 2 calls) use only the readings we have.
        let filled = self.cpu_history_idx.min(3);
        let cpu_percent = self.cpu_history[..filled].iter().sum::<f32>() / filled as f32;

        // RAM
        self.sys.refresh_memory();
        let used = self.sys.used_memory();
        let total = self.sys.total_memory();
        let ram_percent = if total > 0 {
            (used as f32 / total as f32) * 100.0
        } else {
            0.0
        };
        let ram = RamMetrics {
            percent: ram_percent,
            used,
            total,
        };

        // Disks (cached 10 s)
        if self.last_disk_refresh.elapsed().as_secs() >= 10 {
            self.disks.refresh();
            self.disk_cache = collect_disks(&self.disks);
            self.last_disk_refresh = Instant::now();
        }

        // Network delta — per interface
        self.networks.refresh();
        let elapsed = self.last_net_poll.elapsed().as_secs_f64().max(0.001);
        let network = collect_network(&self.networks, &mut self.prev_net, elapsed);
        self.last_net_poll = Instant::now();

        // Top CPU process — refresh CPU-only data for all processes
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::new().with_cpu(),
        );
        let top_cpu = self
            .sys
            .processes()
            .values()
            .filter(|p| p.cpu_usage() > 0.0)
            .max_by(|a, b| {
                a.cpu_usage()
                    .partial_cmp(&b.cpu_usage())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|p| {
                // sysinfo reports cpu_usage per-core (0–100 per core).
                // Divide by core count to get system-relative percentage.
                let pct = (p.cpu_usage() / self.cpu_count).clamp(0.0, 100.0);
                TopProcess {
                    name: p.name().to_string_lossy().to_string(),
                    cpu_percent: pct,
                }
            });

        (cpu_percent, ram, self.disk_cache.clone(), network, top_cpu)
    }
}

/// Returns true for real/useful interfaces — excludes loopback and common
/// Windows virtual adapters (Hyper-V, WSL, VirtualBox, VMware, Bluetooth).
fn is_real_interface(name: &str) -> bool {
    let lo = name.to_lowercase();
    // Loopback
    if lo.contains("loopback") || name == "lo" { return false; }
    // Hyper-V virtual switches (vEthernet (Default Switch), vEthernet (WSL), …)
    if lo.starts_with("vethernet") { return false; }
    // VirtualBox host-only adapters
    if lo.contains("virtualbox") { return false; }
    // VMware virtual adapters
    if lo.contains("vmware") || lo.contains("vmnet") { return false; }
    // Generic virtual / pseudo
    if lo.contains("virtual") || lo.contains("pseudo") { return false; }
    // Bluetooth (rarely useful for bandwidth monitoring)
    if lo.contains("bluetooth") { return false; }
    true
}

/// Compute per-interface upload/download rates (bytes/s).
///
/// Filters:
///   - Virtual/loopback interfaces excluded (see `is_real_interface`).
///   - Interfaces with zero cumulative traffic excluded (never used).
///
/// Sorted by current activity descending. Capped at 3 interfaces.
fn collect_network(
    networks: &Networks,
    prev: &mut HashMap<String, (u64, u64)>,
    elapsed: f64,
) -> Vec<NetworkInterface> {
    let mut result: Vec<NetworkInterface> = networks
        .iter()
        .filter(|(name, data)| {
            let unused = data.total_transmitted() == 0 && data.total_received() == 0;
            is_real_interface(name) && !unused
        })
        .map(|(name, data)| {
            let total_sent = data.total_transmitted();
            let total_recv = data.total_received();

            let (upload, download) = match prev.get(name) {
                Some(&(ps, pr)) => (
                    total_sent.saturating_sub(ps) as f64 / elapsed,
                    total_recv.saturating_sub(pr) as f64 / elapsed,
                ),
                None => (0.0, 0.0),
            };

            prev.insert(name.clone(), (total_sent, total_recv));

            NetworkInterface {
                name: name.clone(),
                upload,
                download,
            }
        })
        .collect();

    // Remove stale entries for interfaces that disappeared (e.g. VPN disconnect)
    prev.retain(|k, _| networks.contains_key(k));

    // Sort: most active first
    result.sort_by(|a, b| {
        (b.upload + b.download)
            .partial_cmp(&(a.upload + a.download))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Cap at 3 to keep the widget compact
    result.truncate(3);
    result
}

fn collect_disks(disks: &Disks) -> Vec<DiskInfo> {
    disks
        .iter()
        .filter_map(|d| {
            let total = d.total_space();
            if total == 0 {
                return None;
            }
            let available = d.available_space();
            let used = total.saturating_sub(available);
            let percent = (used as f32 / total as f32) * 100.0;
            let mount = d.mount_point().to_string_lossy().to_string();
            Some(DiskInfo {
                mount,
                percent,
                used,
                total,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_new_does_not_panic() {
        let _m = Monitor::new();
    }

    #[test]
    fn collect_returns_valid_cpu_percent() {
        let mut m = Monitor::new();
        let (cpu, _, _, _, _) = m.collect();
        assert!((0.0..=100.0).contains(&cpu), "cpu={cpu}");
    }

    #[test]
    fn collect_returns_valid_ram() {
        let mut m = Monitor::new();
        let (_, ram, _, _, _) = m.collect();
        assert!(ram.total > 0, "total RAM should be > 0");
        assert!((0.0..=100.0).contains(&ram.percent), "ram%={}", ram.percent);
        assert!(ram.used <= ram.total);
    }

    #[test]
    fn collect_returns_disks() {
        let mut m = Monitor::new();
        let (_, _, disks, _, _) = m.collect();
        // At least the system drive should appear on any Windows machine.
        assert!(!disks.is_empty(), "expected at least one disk");
        for d in &disks {
            assert!((0.0..=100.0).contains(&d.percent), "disk%={}", d.percent);
            assert!(d.used <= d.total);
        }
    }

    #[test]
    fn collect_returns_network_non_negative() {
        let mut m = Monitor::new();
        // First call initialises the delta counters.
        m.collect();
        // Second call should return a proper delta.
        let (_, _, _, interfaces, _) = m.collect();
        // All rates must be non-negative.
        for iface in &interfaces {
            assert!(iface.upload >= 0.0, "upload < 0 on {}", iface.name);
            assert!(iface.download >= 0.0, "download < 0 on {}", iface.name);
        }
    }
}
