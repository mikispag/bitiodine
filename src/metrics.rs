//! Process memory metrics (Linux only, no external dependencies).

use log::info;

/// Reads `VmHWM` (peak resident set size, "high water mark") from `/proc/self/status`,
/// returned in KiB. Returns 0 if unreadable (e.g. non-Linux platform).
pub fn peak_rss_kib() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find(|line| line.starts_with("VmHWM:"))
                .and_then(|line| {
                    line.split_whitespace()
                        .nth(1)
                        .and_then(|value| value.parse::<u64>().ok())
                })
        })
        .unwrap_or(0)
}

/// Logs the current peak RSS (VmHWM) at info level, tagged with a context string.
pub fn log_peak_rss(context: &str) {
    info!(
        "Peak RSS (VmHWM) {}: {:.1} MiB",
        context,
        peak_rss_kib() as f64 / 1024.0
    );
}
