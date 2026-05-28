use std::time::Instant;
use sysinfo::{Disks, Networks, System};

use crate::models::{DiskInfo, NetworkMetrics, RamMetrics};

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
}

impl Monitor {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let mut networks = Networks::new_with_refreshed_list();
        networks.refresh();
        let (sent, recv) = total_net_bytes(&networks);

        let mut disks = Disks::new_with_refreshed_list();
        disks.refresh();
        let disk_cache = collect_disks(&disks);

        Self {
            sys,
            disks,
            networks,
            prev_sent: sent,
            prev_recv: recv,
            last_net_poll: Instant::now(),
            disk_cache,
            last_disk_refresh: Instant::now(),
        }
    }

    /// Refresh all sensors and return current snapshots.
    /// Disks are re-queried at most once every 10 seconds.
    pub fn collect(&mut self) -> (f32, RamMetrics, Vec<DiskInfo>, NetworkMetrics) {
        // CPU
        self.sys.refresh_cpu_usage();
        let cpu_percent = self
            .sys
            .cpus()
            .iter()
            .map(|c| c.cpu_usage())
            .sum::<f32>()
            / self.sys.cpus().len() as f32;

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

        (cpu_percent, ram, self.disk_cache.clone(), network)
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
        let (cpu, _, _, _) = m.collect();
        assert!((0.0..=100.0).contains(&cpu), "cpu={cpu}");
    }

    #[test]
    fn collect_returns_valid_ram() {
        let mut m = Monitor::new();
        let (_, ram, _, _) = m.collect();
        assert!(ram.total > 0, "total RAM should be > 0");
        assert!((0.0..=100.0).contains(&ram.percent), "ram%={}", ram.percent);
        assert!(ram.used <= ram.total);
    }

    #[test]
    fn collect_returns_disks() {
        let mut m = Monitor::new();
        let (_, _, disks, _) = m.collect();
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
        let (_, _, _, net) = m.collect();
        assert!(net.upload >= 0.0);
        assert!(net.download >= 0.0);
    }
}
