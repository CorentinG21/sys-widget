use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use sysinfo::{Disks, Networks, ProcessRefreshKind, ProcessesToUpdate, System};

use crate::models::{DiskInfo, NetworkMetrics, RamMetrics, TopProcess};
use crate::network::{count_bytes, hardware_interfaces, RateMeter};

/// Hosts tried in order to measure latency. The second one is reached over
/// HTTPS, for networks that block outgoing DNS (port 53) to public resolvers.
const LATENCY_TARGETS: [&str; 2] = ["8.8.8.8:53", "1.1.1.1:443"];

/// Number of measurements (one every 5 s) the reported latency is based on.
const LATENCY_HISTORY: usize = 3;

/// Spawns a background thread that measures TCP latency every 5 s.
/// Returns a shared handle to the latency to display (see `summarize_latency`).
pub fn start_latency_poller() -> Arc<Mutex<Option<u32>>> {
    let shared: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
    let shared_clone = Arc::clone(&shared);

    std::thread::spawn(move || {
        let targets: Vec<SocketAddr> = LATENCY_TARGETS
            .iter()
            .filter_map(|t| t.to_socket_addrs().ok().and_then(|mut it| it.next()))
            .collect();
        let mut history: Vec<Option<u32>> = Vec::with_capacity(LATENCY_HISTORY + 1);

        loop {
            history.push(measure_latency(&targets, Duration::from_secs(2)));
            if history.len() > LATENCY_HISTORY {
                history.remove(0);
            }

            if let Ok(mut guard) = shared_clone.lock() {
                *guard = summarize_latency(&history);
            }

            std::thread::sleep(Duration::from_secs(5));
        }
    });

    shared
}

/// Time to open a TCP connection to the first target that answers, in ms.
fn measure_latency(targets: &[SocketAddr], timeout: Duration) -> Option<u32> {
    targets.iter().find_map(|addr| {
        let start = Instant::now();
        std::net::TcpStream::connect_timeout(addr, timeout)
            .ok()
            .map(|_| start.elapsed().as_millis() as u32)
    })
}

/// Latency to report from the last measurements (oldest first): the median of
/// the successful ones, so one slow sample does not flip the display. Offline
/// (None) only when the last two attempts failed, or the only one did.
fn summarize_latency(history: &[Option<u32>]) -> Option<u32> {
    match history {
        [] | [None] => return None,
        [.., None, None] => return None,
        _ => {}
    }
    let mut ok: Vec<u32> = history.iter().flatten().copied().collect();
    ok.sort_unstable();
    let mid = ok.len() / 2;
    match ok.len() {
        0 => None,
        n if n % 2 == 1 => Some(ok[mid]),
        _ => Some((ok[mid - 1] + ok[mid] + 1) / 2),
    }
}

/// Handles repeated sysinfo polls for CPU, RAM, disks, and network.
pub struct Monitor {
    sys: System,
    disks: Disks,
    networks: Networks,
    /// Smoothed upload/download rates from the byte counters.
    net_rate: RateMeter,
    /// Interfaces counted at the last poll (None = all), to log changes.
    net_interfaces: Option<Vec<String>>,
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

        let networks = Networks::new_with_refreshed_list();

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

        let mut monitor = Self {
            sys,
            disks,
            networks,
            net_rate: RateMeter::default(),
            net_interfaces: None,
            disk_cache,
            last_disk_refresh: Instant::now(),
            cpu_history: [0.0; 3],
            cpu_history_idx: 0,
            cpu_count,
        };
        // First reading: the baseline the first collect() measures from.
        monitor.network_rates();
        monitor
    }

    /// Upload and download in bytes/s over the physical network cards only,
    /// so virtual adapters and VPN tunnels are not added on top (see network.rs).
    fn network_rates(&mut self) -> (f64, f64) {
        // refresh_list(), not refresh(): picks up a card that connects after
        // start (Wi-Fi joining, cable plugged in).
        self.networks.refresh_list();
        let hardware = hardware_interfaces();
        let counted = count_bytes(
            self.networks
                .iter()
                .map(|(n, d)| (n.as_str(), d.total_transmitted(), d.total_received())),
            hardware.as_deref(),
        );
        if counted.interfaces != self.net_interfaces {
            eprintln!(
                "[network] counting {}",
                counted
                    .interfaces
                    .as_ref()
                    .map(|n| n.join(" + "))
                    .unwrap_or_else(|| "all interfaces".into())
            );
            self.net_interfaces = counted.interfaces.clone();
        }
        self.net_rate.update(&counted, Instant::now())
    }

    /// Refresh all sensors and return current snapshots.
    /// Disks are re-queried at most once every 10 seconds.
    pub fn collect(
        &mut self,
        latency: &Arc<Mutex<Option<u32>>>,
    ) -> (
        f32,
        RamMetrics,
        Vec<DiskInfo>,
        NetworkMetrics,
        Option<TopProcess>,
    ) {
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

        // Disks (cached 10 s) — refresh_list() re-discovers new/removed drives (hot-plug)
        if self.last_disk_refresh.elapsed().as_secs() >= 10 {
            self.disks.refresh_list();
            self.disk_cache = collect_disks(&self.disks);
            self.last_disk_refresh = Instant::now();
        }

        // Network
        let (upload, download) = self.network_rates();
        let latency_ms = latency.lock().ok().and_then(|g| *g);
        let network = NetworkMetrics {
            upload,
            download,
            latency_ms,
        };

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
    use crate::models::NetworkMetrics;

    fn no_latency() -> Arc<Mutex<Option<u32>>> {
        Arc::new(Mutex::new(None))
    }

    #[test]
    fn monitor_new_does_not_panic() {
        let _m = Monitor::new();
    }

    #[test]
    fn collect_returns_valid_cpu_percent() {
        let mut m = Monitor::new();
        let (cpu, _, _, _, _) = m.collect(&no_latency());
        assert!((0.0..=100.0).contains(&cpu), "cpu={cpu}");
    }

    #[test]
    fn collect_returns_valid_ram() {
        let mut m = Monitor::new();
        let (_, ram, _, _, _) = m.collect(&no_latency());
        assert!(ram.total > 0, "total RAM should be > 0");
        assert!((0.0..=100.0).contains(&ram.percent), "ram%={}", ram.percent);
        assert!(ram.used <= ram.total);
    }

    #[test]
    fn collect_returns_disks() {
        let mut m = Monitor::new();
        let (_, _, disks, _, _) = m.collect(&no_latency());
        assert!(!disks.is_empty(), "expected at least one disk on Windows");
        for d in &disks {
            assert!((0.0..=100.0).contains(&d.percent), "disk%={}", d.percent);
            assert!(d.used <= d.total);
        }
    }

    #[test]
    fn collect_returns_network_non_negative() {
        let mut m = Monitor::new();
        m.collect(&no_latency());
        let (_, _, _, net, _) = m.collect(&no_latency());
        assert!(net.upload >= 0.0, "upload should be >= 0");
        assert!(net.download >= 0.0, "download should be >= 0");
    }

    #[test]
    fn collect_propagates_latency_from_shared_state() {
        let latency = Arc::new(Mutex::new(Some(42u32)));
        let mut m = Monitor::new();
        let (_, _, _, net, _) = m.collect(&latency);
        assert_eq!(
            net.latency_ms,
            Some(42),
            "latency_ms should match shared state"
        );
    }

    #[test]
    fn collect_propagates_none_latency() {
        let mut m = Monitor::new();
        let (_, _, _, net, _) = m.collect(&no_latency());
        assert_eq!(net.latency_ms, None);
    }

    #[test]
    fn network_metrics_serializes_latency_ms() {
        let net = NetworkMetrics {
            upload: 1.0,
            download: 2.0,
            latency_ms: Some(15),
        };
        let json = serde_json::to_string(&net).unwrap();
        assert!(json.contains("\"latency_ms\":15"), "json={json}");
    }

    #[test]
    fn network_metrics_serializes_null_latency() {
        let net = NetworkMetrics {
            upload: 0.0,
            download: 0.0,
            latency_ms: None,
        };
        let json = serde_json::to_string(&net).unwrap();
        assert!(json.contains("\"latency_ms\":null"), "json={json}");
    }

    // ── Latency ──────────────────────────────────────────────────────────────

    /// A local port that refuses connections right away (nothing listens on it).
    fn closed_port() -> SocketAddr {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        addr
    }

    #[test]
    fn latency_is_measured_on_a_target_that_accepts_connections() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let open = listener.local_addr().unwrap();
        let ms = measure_latency(&[open], Duration::from_secs(1));
        assert!(matches!(ms, Some(v) if v < 1000), "ms={ms:?}");
    }

    #[test]
    fn latency_falls_back_to_the_next_target_when_the_first_is_blocked() {
        // e.g. outgoing port 53 blocked on a company network
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let open = listener.local_addr().unwrap();
        assert!(measure_latency(&[closed_port(), open], Duration::from_secs(1)).is_some());
    }

    #[test]
    fn no_latency_when_no_target_answers() {
        assert_eq!(
            measure_latency(&[closed_port(), closed_port()], Duration::from_secs(1)),
            None
        );
    }

    #[test]
    fn nothing_to_report_before_the_first_measurement() {
        assert_eq!(summarize_latency(&[]), None);
    }

    #[test]
    fn a_single_slow_measurement_does_not_make_the_connection_slow() {
        assert_eq!(
            summarize_latency(&[Some(30), Some(400), Some(40)]),
            Some(40)
        );
    }

    #[test]
    fn two_measurements_are_averaged() {
        assert_eq!(summarize_latency(&[Some(20), Some(40)]), Some(30));
    }

    #[test]
    fn a_single_failed_measurement_does_not_mean_offline() {
        assert_eq!(summarize_latency(&[Some(30), Some(50), None]), Some(40));
        assert_eq!(summarize_latency(&[Some(30), None]), Some(30));
    }

    #[test]
    fn two_failed_measurements_in_a_row_mean_offline() {
        assert_eq!(summarize_latency(&[Some(30), None, None]), None);
    }

    #[test]
    fn offline_right_away_when_the_very_first_measurement_fails() {
        assert_eq!(summarize_latency(&[None]), None);
    }

    #[test]
    fn back_online_as_soon_as_a_measurement_succeeds() {
        assert_eq!(summarize_latency(&[None, None, Some(25)]), Some(25));
    }

    #[test]
    fn latency_poller_starts_without_panic() {
        let shared = start_latency_poller();
        std::thread::sleep(std::time::Duration::from_millis(50));
        // Just verifying the poller thread spawned and the Arc is accessible
        drop(shared.lock().unwrap());
    }
}
