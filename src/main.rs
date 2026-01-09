mod calendar;
mod config;

use config::Config;
use std::collections::BTreeMap;
use zellij_tile::prelude::*;

pub const TIME_TICK_SECS: f64 = 30.0;

macro_rules! log {
    ($($arg:tt)*) => {
        eprintln!("[zj-cal] {}", format!($($arg)*))
    };
}

#[derive(Default)]
struct State {
    events: Vec<calendar::Event>,
    ics_url: String,
    calendar_refresh_ticks: u32,
    error: Option<String>,
    loading: bool,
    permission_granted: bool,
    current_time: String,
    ticks_until_calendar: u32,
    use_12h_time: bool,
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        let config = Config::from(configuration);

        self.ics_url = config.ics_url;
        self.use_12h_time = config.use_12h_time;
        self.calendar_refresh_ticks = (config.refresh_interval_secs / TIME_TICK_SECS).ceil() as u32;
        self.ticks_until_calendar = 0;

        log!(
            "load() ics_url={}, refresh_interval={}s (every {} ticks)",
            if self.ics_url.is_empty() { "unset" } else { "[REDACTED]" },
            config.refresh_interval_secs,
            self.calendar_refresh_ticks
        );

        request_permission(&[PermissionType::RunCommands]);

        subscribe(&[
            EventType::Timer,
            EventType::RunCommandResult,
            EventType::PermissionRequestResult,
        ]);
    }

    fn update(&mut self, _event: Event) -> bool {
        false
    }

    fn render(&mut self, _rows: usize, _cols: usize) {
        println!("zj-cal");
    }
}
