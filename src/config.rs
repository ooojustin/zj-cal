use std::collections::BTreeMap;

pub const DEFAULT_REFRESH_INTERVAL_SECS: f64 = 300.0;
pub const DEFAULT_USE_12H_TIME: bool = true;

pub struct Config {
    pub ics_url: String,
    pub refresh_interval_secs: f64,
    pub use_12h_time: bool,
}

impl From<BTreeMap<String, String>> for Config {
    fn from(map: BTreeMap<String, String>) -> Self {
        Self {
            ics_url: map.get("ics_url").cloned().unwrap_or_default(),
            refresh_interval_secs: map
                .get("refresh_interval")
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_REFRESH_INTERVAL_SECS),
            use_12h_time: map
                .get("time_format")
                .map(|s| s != "24")
                .unwrap_or(DEFAULT_USE_12H_TIME),
        }
    }
}
