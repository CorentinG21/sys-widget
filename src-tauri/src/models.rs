use serde::{Deserialize, Serialize};

/// CPU metrics snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuMetrics {
    /// Usage percentage 0–100.
    pub percent: f32,
    /// Temperature in °C from LHM, None if unavailable.
    pub temp: Option<f32>,
}

/// GPU metrics from LHM (single GPU or first discrete GPU).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuMetrics {
    pub name: String,
    /// Load percentage 0–100.
    pub percent: f32,
    /// Core temperature in °C.
    pub temp: Option<f32>,
    /// Used VRAM in bytes.
    pub vram_used: u64,
    /// Total VRAM in bytes.
    pub vram_total: u64,
}

/// RAM metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RamMetrics {
    /// Usage percentage 0–100.
    pub percent: f32,
    /// Used memory in bytes.
    pub used: u64,
    /// Total memory in bytes.
    pub total: u64,
}

/// Single disk / partition metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    /// Mount point (e.g. "C:\\").
    pub mount: String,
    /// Usage percentage 0–100.
    pub percent: f32,
    /// Used space in bytes.
    pub used: u64,
    /// Total space in bytes.
    pub total: u64,
}

/// Network delta metrics (bytes per second since last poll).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    /// Upload bytes/s.
    pub upload: f64,
    /// Download bytes/s.
    pub download: f64,
}

/// Data received from the LHM PowerShell subprocess.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LhmData {
    pub cpu_temp: Option<f32>,
    pub gpu: Option<GpuMetrics>,
}

/// Full payload emitted to the frontend on every tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsPayload {
    pub cpu: CpuMetrics,
    /// None when no GPU is detected.
    pub gpu: Option<GpuMetrics>,
    pub ram: RamMetrics,
    pub disks: Vec<DiskInfo>,
    pub network: NetworkMetrics,
}
