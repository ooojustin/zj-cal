#[macro_use]
mod macros;
mod calendar;
mod config;
use config::Config;
use owo_colors::OwoColorize;
use std::collections::BTreeMap;
use zellij_tile::prelude::*;

pub const TIME_TICK_SECS: f64 = 30.0;
const DEBUG_SAVE_ICS: bool = true;

define_ctx! {
    TimeFetch => "time_fetch",
    IcsFetch => "ics_fetch",
    IcsFetchFile { path: String } => "ics_fetch_file",
    IcsReadFile { path: String } => "ics_read_file",
}

#[derive(Default)]
struct State {
    events: Vec<calendar::Event>,
    ics_url: String,
    calendar_refresh_ticks: u32, // Fetch calendar every N time ticks
    error: Option<String>,
    loading: bool,
    permission_granted: bool,
    current_time: String, // Format: "YYYY-MM-DD HH:MM"
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
        self.ticks_until_calendar = 0; // Fetch immediately on first tick

        log!(
            "load() ics_url={}, refresh_interval={}s (every {} ticks)",
            if self.ics_url.is_empty() {
                "unset"
            } else {
                "[REDACTED]"
            },
            config.refresh_interval_secs,
            self.calendar_refresh_ticks
        );

        // Request necessary permissions
        request_permission(&[PermissionType::RunCommands]);

        // Subscribe to events
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
                    // Use a short delay to let permission system fully initialize
                    // This works around a race condition in Zellij
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
            Event::RunCommandResult(exit_code, stdout, stderr, ctx) => {
                match Ctx::from_map(&ctx) {
                    Ok(Ctx::TimeFetch) => {
                        self.handle_time_fetch(exit_code, stdout, stderr);
                    }
                    Ok(Ctx::IcsFetch) => {
                        self.handle_ics_fetch(exit_code, stdout, stderr);
                    }
                    Ok(Ctx::IcsFetchFile { path }) => {
                        self.handle_ics_fetch_file(exit_code, stderr, path);
                    }
                    Ok(Ctx::IcsReadFile { .. }) => {
                        self.handle_ics_read_file(exit_code, stdout, stderr);
                    }
                    Err(err) => {
                        log!("Invalid context: {}", err);
                    }
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

        // Header - show time as soon as we have it, with optional loading indicator
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

        // Error display
        if let Some(ref err) = self.error {
            println!("{}", truncate(err, width).red());
            return;
        }

        // Events
        if self.events.is_empty() {
            println!("{}", "No upcoming events".dimmed());
            return;
        }

        // Reserve: 1 header + 1 separator + 1 "+more" + 1 buffer for floating mode
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
        log!("fetch_time() - getting current time");
        self.loading = true;

        run_command(&["date", "+%Y-%m-%d %H:%M"], Ctx::TimeFetch.into_map());
    }

    fn fetch_calendar(&mut self) {
        if self.ics_url.is_empty() {
            return;
        }

        let mut curl_args = vec!["curl".to_string(), "-sSfL".to_string()];

        let ctx = if DEBUG_SAVE_ICS {
            let timestamp = self.current_time.replace([':', ' '], "-");
            let path = format!("/tmp/zj-cal/{}.ics", timestamp);
            log!("fetch_calendar() - saving to {}", path);
            curl_args.push("--create-dirs".to_string());
            curl_args.push("--output".to_string());
            curl_args.push(path.clone());
            Ctx::IcsFetchFile { path }
        } else {
            log!("fetch_calendar()");
            Ctx::IcsFetch
        };

        curl_args.push("--".to_string());
        curl_args.push(self.ics_url.clone());

        let curl_args_ref: Vec<&str> = curl_args.iter().map(|s| s.as_str()).collect();
        run_command(&curl_args_ref, ctx.into_map());
    }

    fn handle_ics_output(
        &mut self,
        exit_code: Option<i32>,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        action_label: &str,
        error_label: &str,
    ) {
        self.loading = false;
        if exit_code == Some(0) {
            log!("{} ({} bytes)", action_label, stdout.len());
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
            self.error = Some(format!("{}: {}", error_label, err_msg));
        }
    }

    fn handle_ics_fetch(&mut self, exit_code: Option<i32>, stdout: Vec<u8>, stderr: Vec<u8>) {
        self.handle_ics_output(exit_code, stdout, stderr, "Fetched ICS", "Fetch failed");
    }

    fn handle_ics_read_file(&mut self, exit_code: Option<i32>, stdout: Vec<u8>, stderr: Vec<u8>) {
        self.handle_ics_output(exit_code, stdout, stderr, "Read ICS", "Read failed");
    }

    fn handle_time_fetch(&mut self, exit_code: Option<i32>, stdout: Vec<u8>, stderr: Vec<u8>) {
        if exit_code == Some(0) {
            self.current_time = String::from_utf8_lossy(&stdout).trim().to_string();
            log!("Current time: {}", self.current_time);

            // Fetch calendar when counter reaches 0
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

    fn handle_ics_fetch_file(&mut self, exit_code: Option<i32>, stderr: Vec<u8>, path: String) {
        if exit_code == Some(0) {
            let read_ctx = Ctx::IcsReadFile { path: path.clone() }.into_map();
            run_command(&["cat", path.as_str()], read_ctx);
        } else {
            self.loading = false;
            let err_msg = String::from_utf8_lossy(&stderr);
            self.error = Some(format!("Fetch failed: {}", err_msg));
        }
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
