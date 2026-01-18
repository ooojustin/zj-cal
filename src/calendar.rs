use chrono::NaiveDateTime;
use icalendar::{Calendar, CalendarComponent, Component, DatePerhapsTime, EventLike};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Event {
    pub summary: String,
    pub start: String,
    pub end: String,
    pub location: Option<String>,
}

impl Event {
    pub fn is_video_call(&self) -> bool {
        self.location
            .as_ref()
            .map(|l| l.contains("zoom") || l.contains("meet.google") || l.contains("teams"))
            .unwrap_or(false)
    }
}

pub fn parse_ics(data: &[u8]) -> Result<Vec<Event>, String> {
    let content = String::from_utf8_lossy(data);

    let calendar: Calendar = content.parse().map_err(|e| format!("Parse error: {}", e))?;

    let events: Vec<Event> = calendar
        .components
        .iter()
        .filter_map(|component| {
            if let CalendarComponent::Event(event) = component {
                let summary = event.get_summary().unwrap_or("(no title)").to_string();
                let start = event.get_start().map(format_date_perhaps_time)?;
                let end = event.get_end().map(format_date_perhaps_time);
                let location = event.get_location().map(|s| s.to_string());

                Some(Event {
                    summary,
                    start,
                    end: end.unwrap_or_default(),
                    location,
                })
            } else {
                None
            }
        })
        .collect();

    Ok(events)
}

pub fn filter_upcoming(mut events: Vec<Event>, current_time: &str, limit: usize) -> Vec<Event> {
    events.sort_by(|a, b| a.start.cmp(&b.start));

    if !current_time.is_empty() {
        events.retain(|e| e.start.as_str() >= current_time);
    }

    events.truncate(limit);
    events
}

fn format_date_perhaps_time(dt: DatePerhapsTime) -> String {
    use icalendar::CalendarDateTime;

    match dt {
        DatePerhapsTime::DateTime(cdt) => {
            let naive = match cdt {
                CalendarDateTime::Floating(dt) => dt,
                CalendarDateTime::Utc(dt) => dt.naive_utc(),
                CalendarDateTime::WithTimezone { date_time, .. } => date_time,
            };
            format!("{}", naive.format("%Y-%m-%d %H:%M"))
        }
        DatePerhapsTime::Date(date) => {
            format!("{} 00:00", date)
        }
    }
}

pub fn format_time(time_24h: &str, use_12h: bool) -> String {
    if !use_12h || time_24h.len() < 5 {
        return time_24h.to_string();
    }

    let hour: u32 = time_24h[0..2].parse().unwrap_or(0);
    let min = &time_24h[3..5];

    let (hour_12, period) = match hour {
        0 => (12, "am"),
        1..=11 => (hour, "am"),
        12 => (12, "pm"),
        _ => (hour - 12, "pm"),
    };

    format!("{}:{} {}", hour_12, min, period)
}

pub fn format_datetime_display(dt: &str, use_12h: bool) -> String {
    if dt.len() >= 16 {
        let month = &dt[5..7];
        let day = &dt[8..10];
        let time = &dt[11..16];

        let month_name = match month {
            "01" => "jan",
            "02" => "feb",
            "03" => "mar",
            "04" => "apr",
            "05" => "may",
            "06" => "jun",
            "07" => "jul",
            "08" => "aug",
            "09" => "sep",
            "10" => "oct",
            "11" => "nov",
            "12" => "dec",
            _ => month,
        };

        if time == "00:00" {
            format!("{} {}", month_name, day.trim_start_matches('0'))
        } else {
            let formatted_time = format_time(time, use_12h);
            format!(
                "{} {} {}",
                month_name,
                day.trim_start_matches('0'),
                formatted_time
            )
        }
    } else {
        dt.to_string()
    }
}

fn parse_datetime(dt: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(dt, "%Y-%m-%d %H:%M").ok()
}

/// "9:00 am" -> "9am", "9:30 am" -> "9:30am"
fn compact_time(time: &str) -> String {
    time.replace(":00 ", "").replace(' ', "")
}

/// Formats an event time relative to the current time.
pub fn format_event_time(event_time: &str, current_time: &str, use_12h: bool) -> String {
    let Some(event_dt) = parse_datetime(event_time) else {
        // Failed to parse event time, fallback to absolute
        return format_datetime_display(event_time, use_12h);
    };
    let Some(now_dt) = parse_datetime(current_time) else {
        // Failed to parse current time, fallback to absolute
        return format_datetime_display(event_time, use_12h);
    };

    let minutes = event_dt.signed_duration_since(now_dt).num_minutes();

    // Past events or >24h away: absolute format
    if !(0..=24 * 60).contains(&minutes) {
        return format_datetime_display(event_time, use_12h);
    }

    let is_tomorrow = event_dt.date() != now_dt.date();
    let time_part = &event_time[11..16];
    let is_all_day = time_part == "00:00";

    // All-day events get date-only labels, not relative time
    if is_all_day {
        return if is_tomorrow {
            "tmrw".to_string()
        } else {
            "today".to_string()
        };
    }

    match minutes {
        0 => "now".to_string(),
        1..=59 => format!("in {} min", minutes),
        60..=299 => {
            let whole_hours = minutes / 60;
            let remainder = minutes % 60;
            // Show .5 if within 10 min of half hour (20-40 min past)
            if (20..=40).contains(&remainder) {
                format!("in {}.5 hrs", whole_hours)
            } else if remainder > 40 {
                // Round up
                format!("in {} hrs", whole_hours + 1)
            } else if whole_hours == 1 {
                "in 1 hr".to_string()
            } else {
                format!("in {} hrs", whole_hours)
            }
        }
        _ if is_tomorrow => {
            format!("tmrw {}", compact_time(&format_time(time_part, use_12h)))
        }
        _ => {
            format!("today {}", compact_time(&format_time(time_part, use_12h)))
        }
    }
}
