mod calendar;
mod config;

use config::Config;
use owo_colors::OwoColorize;
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

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(status) => {
                log!("PermissionRequestResult: {:?}", status);
                if status == PermissionStatus::Granted && !self.permission_granted {
                    self.permission_granted = true;
                    log!("Permission granted, scheduling fetch...");
                    set_timeout(0.1);
                } else if status != PermissionStatus::Granted {
                    log!("Permission NOT granted");
                }
                true
            }
            Event::Timer(_) => {
                self.fetch_time();
                set_timeout(TIME_TICK_SECS);
                true
            }
            Event::RunCommandResult(exit_code, stdout, stderr, context) => {
                match context.get("source").map(|s| s.as_str()) {
                    Some("time_fetch") => {
                        if exit_code == Some(0) {
                            self.current_time = String::from_utf8_lossy(&stdout).trim().to_string();
                            log!("Current time: {}", self.current_time);

                            if self.ticks_until_calendar == 0 {
                                self.ticks_until_calendar = self.calendar_refresh_ticks;
                                self.fetch_calendar();
                            } else {
                                self.ticks_until_calendar -= 1;
                                self.loading = false;
                            }
                        } else {
                            log!("Failed to get time: {}", String::from_utf8_lossy(&stderr));
                            self.loading = false;
                        }
                    }
                    Some("ics_fetch") => {
                        self.loading = false;
                        if exit_code == Some(0) {
                            log!("Fetched ICS ({} bytes)", stdout.len());
                            match calendar::parse_ics(&stdout) {
                                Ok(events) => {
                                    self.events = calendar::filter_upcoming(events, &self.current_time, 20);
                                    self.error = None;
                                }
                                Err(e) => {
                                    log!("Failed to parse ICS: {}", e);
                                    self.error = Some(e);
                                }
                            }
                        } else {
                            let err_msg = String::from_utf8_lossy(&stderr);
                            self.error = Some(format!("Fetch failed: {}", err_msg));
                        }
                    }
                    _ => {}
                }
                true
            }
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        let width = cols.min(50);

        if self.ics_url.is_empty() {
            println!("{}", "⚠ No ICS URL configured".yellow());
            println!();
            println!("Add to your plugin config:");
            println!("  ics_url \"https://...\"");
            return;
        }

        print!("{} ", "📅 Calendar".blue().bold());
        if self.current_time.len() >= 16 {
            let time_str = calendar::format_time(&self.current_time[11..16], self.use_12h_time);
            print!("{}", time_str.dimmed());
            if self.loading {
                println!(" {}", "↻".yellow());
            } else {
                println!();
            }
        } else if self.loading {
            println!("{}", "↻".yellow());
        } else {
            println!();
        }
        println!("{}", "─".repeat(width));

        if let Some(ref err) = self.error {
            println!("{}", truncate(err, width).red());
            return;
        }

        if self.events.is_empty() {
            println!("{}", "No upcoming events".dimmed());
            return;
        }

        let max_events = rows.saturating_sub(4);
        for event in self.events.iter().take(max_events) {
            let time = calendar::format_datetime_display(&event.start, self.use_12h_time);
            let summary = truncate(&event.summary, width.saturating_sub(time.len() + 3));
            let icon = if event.is_video_call() { "📹" } else { "•" };
            println!("{} {} {}", time.cyan(), icon, summary);
        }

        if self.events.len() > max_events {
            println!(
                "{}",
                format!("  +{} more", self.events.len() - max_events).dimmed()
            );
        }
    }
}

impl State {
    fn fetch_time(&mut self) {
        log!("fetch_time()");
        self.loading = true;

        let mut context = BTreeMap::new();
        context.insert("source".to_string(), "time_fetch".to_string());

        run_command(&["date", "+%Y-%m-%d %H:%M"], context);
    }

    fn fetch_calendar(&mut self) {
        if self.ics_url.is_empty() {
            return;
        }

        log!("fetch_calendar()");

        let mut context = BTreeMap::new();
        context.insert("source".to_string(), "ics_fetch".to_string());

        run_command(&["curl", "-sSfL", "--", &self.ics_url], context);
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}
