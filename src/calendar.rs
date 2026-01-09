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
            "01" => "Jan",
            "02" => "Feb",
            "03" => "Mar",
            "04" => "Apr",
            "05" => "May",
            "06" => "Jun",
            "07" => "Jul",
            "08" => "Aug",
            "09" => "Sep",
            "10" => "Oct",
            "11" => "Nov",
            "12" => "Dec",
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
