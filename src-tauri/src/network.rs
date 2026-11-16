//! Network throughput: which interfaces to count, and a smoothed rate.

use std::time::Instant;

/// Time constant of the smoothing: a change shows at ~63 % after this long.
const SMOOTHING_SECS: f64 = 2.0;

/// Bytes counted on one tick, and which interfaces they come from
/// (`None` = the sum of all interfaces, the fallback).
#[derive(Debug, Clone, PartialEq)]
pub struct Counted {
    pub interfaces: Option<Vec<String>>,
    pub sent: u64,
    pub received: u64,
}

/// Names (aliases, as sysinfo names them) of the physical network cards:
/// Ethernet and Wi-Fi, connected or not. None when the lookup fails.
///
/// Counting only these leaves out virtual adapters (VirtualBox, Hyper-V, WSL),
/// whose traffic never leaves the machine or already goes through a card, and
/// VPN tunnels, whose traffic is already counted on the card they run over.
/// All cards are kept, not just the default route's: with Ethernet and Wi-Fi
/// both connected, traffic can go through either.
#[cfg(windows)]
pub fn hardware_interfaces() -> Option<Vec<String>> {
    use windows_sys::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2};

    /// MIB_IF_ROW2.InterfaceAndOperStatusFlags bits.
    const HARDWARE_INTERFACE: u8 = 0b01;
    const FILTER_INTERFACE: u8 = 0b10;

    let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    // SAFETY: `table` is a valid out pointer; on success Windows allocates the
    // table, which is freed below with FreeMibTable as documented.
    if unsafe { GetIfTable2(&mut table) } != 0 || table.is_null() {
        return None;
    }
    // SAFETY: on success `table` points to NumEntries consecutive rows.
    let rows = unsafe {
        std::slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize)
    };
    let mut names: Vec<String> = rows
        .iter()
        .filter(|row| {
            let flags = row.InterfaceAndOperStatusFlags._bitfield;
            // Filter interfaces are the driver layers stacked on a card (QoS,
            // WFP): same hardware, not a second card.
            flags & HARDWARE_INTERFACE != 0 && flags & FILTER_INTERFACE == 0
        })
        .filter_map(|row| {
            let len = row.Alias.iter().position(|&c| c == 0).unwrap_or(row.Alias.len());
            String::from_utf16(&row.Alias[..len]).ok()
        })
        .collect();
    // SAFETY: `table` was allocated by GetIfTable2 and is not used after this.
    unsafe { FreeMibTable(table as *const core::ffi::c_void) };

    names.sort();
    names.dedup();
    Some(names)
}

#[cfg(not(windows))]
pub fn hardware_interfaces() -> Option<Vec<String>> {
    None
}

/// Totals over the interfaces in `only` that are listed, or over all
/// interfaces when `only` is None or matches none of them.
pub fn count_bytes<'a>(
    interfaces: impl IntoIterator<Item = (&'a str, u64, u64)>,
    only: Option<&[String]>,
) -> Counted {
    let list: Vec<(&str, u64, u64)> = interfaces.into_iter().collect();
    let sum = |rows: &[&(&str, u64, u64)]| {
        rows.iter()
            .fold((0u64, 0u64), |(s, r), (_, ds, dr)| (s + ds, r + dr))
    };

    if let Some(wanted) = only {
        let picked: Vec<&(&str, u64, u64)> = list
            .iter()
            .filter(|(name, _, _)| wanted.iter().any(|w| w == name))
            .collect();
        if !picked.is_empty() {
            let (sent, received) = sum(&picked);
            let mut names: Vec<String> = picked.iter().map(|(n, _, _)| n.to_string()).collect();
            names.sort();
            return Counted {
                interfaces: Some(names),
                sent,
                received,
            };
        }
    }

    let (sent, received) = sum(&list.iter().collect::<Vec<_>>());
    Counted {
        interfaces: None,
        sent,
        received,
    }
}

/// Turns cumulative byte counters into smoothed bytes-per-second rates.
#[derive(Default)]
pub struct RateMeter {
    /// Previous reading: interfaces, counters and time.
    last: Option<(Counted, Instant)>,
    up: f64,
    down: f64,
}

impl RateMeter {
    /// Returns (upload, download) in bytes per second.
    pub fn update(&mut self, counted: &Counted, now: Instant) -> (f64, f64) {
        let previous = self.last.replace((counted.clone(), now));
        let Some((before, then)) = previous else {
            return (0.0, 0.0);
        };
        // Other interfaces: their counters cannot be compared with the old ones.
        // Keep the current rates; the next reading measures the new set.
        if before.interfaces != counted.interfaces {
            return (self.up, self.down);
        }
        let dt = now.saturating_duration_since(then).as_secs_f64();
        if dt <= 0.0 {
            return (self.up, self.down);
        }
        // saturating_sub: counters restart from zero when an adapter resets.
        let up = counted.sent.saturating_sub(before.sent) as f64 / dt;
        let down = counted.received.saturating_sub(before.received) as f64 / dt;
        // Exponential smoothing, weighted by the time elapsed, so it behaves
        // the same at every polling interval.
        let alpha = 1.0 - (-dt / SMOOTHING_SECS).exp();
        self.up += alpha * (up - self.up);
        self.down += alpha * (down - self.down);
        (self.up, self.down)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn all(sent: u64, received: u64) -> Counted {
        Counted { interfaces: None, sent, received }
    }

    fn on(names: &[&str], sent: u64, received: u64) -> Counted {
        Counted {
            interfaces: Some(names.iter().map(|n| n.to_string()).collect()),
            sent,
            received,
        }
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|n| n.to_string()).collect()
    }

    // ── Which interfaces ─────────────────────────────────────────────────────

    // As on a real machine: Ethernet and Wi-Fi both connected, plus virtual ones.
    const IFACES: [(&str, u64, u64); 4] = [
        ("Ethernet 5", 1_000, 50_000),   // network card
        ("Wi-Fi", 200, 3_000),           // network card
        ("Ethernet 4", 300, 9_000),      // VirtualBox host-only
        ("Tailscale", 700, 40_000),      // VPN tunnel, already counted on a card
    ];

    #[test]
    fn ethernet_and_wifi_connected_together_are_both_counted() {
        let hardware = names(&["Ethernet 5", "Wi-Fi"]);
        assert_eq!(
            count_bytes(IFACES, Some(&hardware)),
            on(&["Ethernet 5", "Wi-Fi"], 1_200, 53_000)
        );
    }

    #[test]
    fn virtual_adapters_and_vpn_tunnels_are_not_counted() {
        let hardware = names(&["Ethernet 5"]);
        assert_eq!(
            count_bytes(IFACES, Some(&hardware)),
            on(&["Ethernet 5"], 1_000, 50_000)
        );
    }

    #[test]
    fn cards_that_are_not_connected_are_simply_skipped() {
        // sysinfo only lists connected interfaces; a card unplugged is absent.
        let hardware = names(&["Ethernet 5", "Ethernet 2"]);
        assert_eq!(
            count_bytes(IFACES, Some(&hardware)),
            on(&["Ethernet 5"], 1_000, 50_000)
        );
    }

    #[test]
    fn the_counted_names_do_not_depend_on_the_listing_order() {
        let a = count_bytes(IFACES, Some(&names(&["Wi-Fi", "Ethernet 5"])));
        let reversed: Vec<_> = IFACES.iter().rev().copied().collect();
        let b = count_bytes(reversed, Some(&names(&["Ethernet 5", "Wi-Fi"])));
        assert_eq!(a, b);
    }

    #[test]
    fn all_interfaces_are_summed_when_the_cards_are_unknown() {
        assert_eq!(count_bytes(IFACES, None), all(2_200, 102_000));
    }

    #[test]
    fn all_interfaces_are_summed_when_no_card_is_listed() {
        let hardware = names(&["Ethernet 2"]);
        assert_eq!(count_bytes(IFACES, Some(&hardware)), all(2_200, 102_000));
    }

    #[test]
    fn a_connected_network_card_is_recognised_by_name() {
        // sysinfo lists the connected interfaces. If none of them were found
        // among the hardware ones, the widget would silently count everything.
        let Some(hardware) = hardware_interfaces() else {
            return;
        };
        let networks = sysinfo::Networks::new_with_refreshed_list();
        let listed: Vec<&str> = networks.iter().map(|(n, _)| n.as_str()).collect();
        assert!(
            listed.is_empty() || hardware.iter().any(|h| listed.contains(&h.as_str())),
            "hardware {hardware:?}, sysinfo {listed:?}"
        );
    }

    // ── Rates ────────────────────────────────────────────────────────────────

    /// Feeds `(seconds since start, counted)` ticks and returns the last rates.
    fn run(ticks: &[(u64, Counted)]) -> (f64, f64) {
        let start = Instant::now();
        let mut meter = RateMeter::default();
        let mut last = (0.0, 0.0);
        for (secs, counted) in ticks {
            last = meter.update(counted, start + Duration::from_secs(*secs));
        }
        last
    }

    #[test]
    fn nothing_is_reported_before_a_second_reading() {
        assert_eq!(run(&[(0, all(5_000_000, 9_000_000))]), (0.0, 0.0));
    }

    #[test]
    fn a_steady_rate_is_reported_after_a_few_seconds() {
        // 1000 B/s up and 4000 B/s down, read every 2 s.
        let ticks: Vec<(u64, Counted)> =
            (0..=5).map(|i| (i * 2, all(i * 2_000, i * 8_000))).collect();
        let (up, down) = run(&ticks);
        assert!((up - 1_000.0).abs() < 50.0, "up={up}");
        assert!((down - 4_000.0).abs() < 200.0, "down={down}");
    }

    #[test]
    fn a_short_burst_shows_quickly_but_damped() {
        // Idle, then 2 s at 100 kB/s.
        let (_, down) = run(&[(0, all(0, 0)), (2, all(0, 0)), (4, all(0, 200_000))]);
        assert!(down > 50_000.0 && down < 100_000.0, "down={down}");
    }

    #[test]
    fn the_rate_falls_back_quickly_once_traffic_stops() {
        let (_, down) = run(&[
            (0, all(0, 0)),
            (2, all(0, 200_000)),
            (4, all(0, 200_000)),
            (6, all(0, 200_000)),
        ]);
        assert!(down < 20_000.0, "down={down}");
    }

    #[test]
    fn a_card_appearing_does_not_create_a_spike() {
        // Wi-Fi connects: its whole counter must not count as traffic.
        let (up, down) = run(&[
            (0, on(&["Ethernet 5"], 1_000_000, 2_000_000)),
            (2, on(&["Ethernet 5"], 1_002_000, 2_004_000)),
            (4, on(&["Ethernet 5", "Wi-Fi"], 9_000_000_000, 9_000_000_000)),
        ]);
        assert!(up <= 1_000.0 && down <= 2_000.0, "up={up} down={down}");
    }

    #[test]
    fn the_new_set_of_cards_is_measured_from_the_reading_after_the_change() {
        let (_, down) = run(&[
            (0, all(0, 0)),
            (2, on(&["Wi-Fi"], 0, 1_000_000)),
            (4, on(&["Wi-Fi"], 0, 1_000_000)),
        ]);
        assert!(down < 1.0, "down={down}");
    }

    #[test]
    fn counters_that_go_back_down_count_as_no_traffic() {
        // An adapter reset restarts its counters from zero.
        let (up, down) = run(&[(0, all(500_000, 900_000)), (2, all(10, 20))]);
        assert_eq!((up, down), (0.0, 0.0));
    }

    #[test]
    fn two_readings_at_the_same_instant_do_not_divide_by_zero() {
        let (up, down) = run(&[(0, all(0, 0)), (0, all(1_000, 1_000))]);
        assert!(up.is_finite() && down.is_finite(), "up={up} down={down}");
    }
}
