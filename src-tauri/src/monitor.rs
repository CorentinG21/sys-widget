use std::time::Instant;
use sysinfo::{Disks, Networks, ProcessRefreshKind, ProcessesToUpdate, System};

use crate::models::{DiskInfo, NetworkMetrics, RamMetrics, TopProcess};

/// Handles repeated sysinfo polls for CPU, RAM, disks, and network.
pub struct Monitor {
    sys: System,
    disks: Disks,
    networks: Networks,
    /// Absolute bytes sent/received at the last poll, used to compute per-second deltas.
    prev_sent: u64,
    prev_recv: u64,
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
        let (sent, recv) = total_net_bytes(&networks);

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
            prev_sent: sent,
            prev_recv: recv,
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
    pub fn collect(&mut self) -> (f32, RamMetrics, Vec<DiskInfo>, NetworkMetrics, Option<TopProcess>) {
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

        // Network delta
        self.networks.refresh();
        let (sent, recv) = total_net_bytes(&self.networks);
        let elapsed = self.last_net_poll.elapsed().as_secs_f64().max(0.001);
        let upload = (sent.saturating_sub(self.prev_sent) as f64) / elapsed;
        let download = (recv.saturating_sub(self.prev_recv) as f64) / elapsed;
        self.prev_sent = sent;
        self.prev_recv = recv;
        self.last_net_poll = Instant::now();
        let network = NetworkMetrics { upload, download };

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

fn total_net_bytes(networks: &Networks) -> (u64, u64) {
    networks
        .iter()
        .fold((0, 0), |(s, r), (_, data)| {
            (s + data.total_transmitted(), r + data.total_received())
        })
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
        let (_, _, _, net, _) = m.collect();
        assert!(net.upload >= 0.0);
        assert!(net.download >= 0.0);
    }
}
