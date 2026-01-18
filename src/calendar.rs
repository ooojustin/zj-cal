use chrono::{NaiveDate, NaiveDateTime, Timelike};
use icalendar::CalendarDateTime;
use icalendar::{Calendar, CalendarComponent, Component, DatePerhapsTime, EventLike};

const DATETIME_FMT: &str = "%Y-%m-%d %H:%M";

pub struct Event {
    pub summary: String,
    pub start: NaiveDateTime,
    #[allow(dead_code)]
    pub end: Option<NaiveDateTime>,
    pub location: Option<String>,
    pub is_all_day: bool,
}

impl Event {
    pub fn is_video_call(&self) -> bool {
        self.location
            .as_ref()
            .map(|l| l.contains("zoom") || l.contains("meet.google") || l.contains("teams"))
            .unwrap_or(false)
    }

    /// Returns true if the event is currently in progress (started and not ended).
    pub fn is_in_progress(&self, now: NaiveDateTime) -> bool {
        self.end.is_some_and(|end| self.start <= now && now < end)
    }

    /// Returns true if the event should be considered active on the given date.
    pub fn is_active_on(&self, date: NaiveDate) -> bool {
        let start_date = self.start.date();
        match self.end {
            Some(end) if self.is_all_day => start_date <= date && date < end.date(),
            Some(end) => {
                // Timed event: active if it overlaps the date (handles overnight events)
                let day_start = date.and_hms_opt(0, 0, 0).unwrap();
                start_date <= date && end > day_start
            }
            None => start_date == date,
        }
    }
}

/// Parses ICS calendar data into a list of events.
pub fn parse_ics(data: &[u8], utc_offset_minutes: i32) -> Result<Vec<Event>, String> {
    let content = String::from_utf8_lossy(data);
    let calendar: Calendar = content.parse().map_err(|e| format!("Parse error: {}", e))?;

    let events: Vec<Event> = calendar
        .components
        .iter()
        .filter_map(|component| {
            if let CalendarComponent::Event(event) = component {
                let summary = event.get_summary().unwrap_or("(no title)").to_string();
                let start_raw = event.get_start()?;
                let is_all_day = matches!(&start_raw, DatePerhapsTime::Date(_));
                let start = parse_date_perhaps_time(start_raw, utc_offset_minutes);
                let end = event
                    .get_end()
                    .map(|dt| parse_date_perhaps_time(dt, utc_offset_minutes));
                let location = event.get_location().map(|s| s.to_string());

                Some(Event {
                    summary,
                    start,
                    end,
                    location,
                    is_all_day,
                })
            } else {
                None
            }
        })
        .collect();

    Ok(events)
}

/// Removes past events (keeps in-progress), sorts by start time, truncates to `limit`.
pub fn filter_future(
    mut events: Vec<Event>,
    current_time: Option<NaiveDateTime>,
    limit: usize,
) -> Vec<Event> {
    events.sort_by(|a, b| a.start.cmp(&b.start));
    if let Some(now) = current_time {
        events.retain(|e| e.start >= now || e.end.is_some_and(|end| end > now));
    }
    events.truncate(limit);
    events
}

/// Converts ICS DatePerhapsTime to NaiveDateTime in local time.
/// All-day events get 00:00.
///
/// Note: UTC offset is based on current time, not event time. Events crossing a DST
/// boundary may be off by 1 hour. Acceptable for a near-term calendar widget.
fn parse_date_perhaps_time(dt: DatePerhapsTime, utc_offset_minutes: i32) -> NaiveDateTime {
    match dt {
        DatePerhapsTime::DateTime(cdt) => match cdt {
            CalendarDateTime::Floating(dt) => dt,
            CalendarDateTime::Utc(dt) => {
                dt.naive_utc() + chrono::Duration::minutes(utc_offset_minutes as i64)
            }
            CalendarDateTime::WithTimezone { date_time, .. } => date_time,
        },
        DatePerhapsTime::Date(date) => date.and_hms_opt(0, 0, 0).unwrap(),
    }
}

/// Parses UTC offset string (e.g., "-0500", "+0530") to minutes.
pub fn parse_utc_offset(s: &str) -> Option<i32> {
    let s = s.trim();
    if s.len() != 5 {
        return None;
    }
    let sign = match s.chars().next()? {
        '+' => 1,
        '-' => -1,
        _ => return None,
    };
    let hours: i32 = s[1..3].parse().ok()?;
    let minutes: i32 = s[3..5].parse().ok()?;
    Some(sign * (hours * 60 + minutes))
}

/// Parses "YYYY-MM-DD HH:MM" string (from shell `date` command) to NaiveDateTime.
pub fn parse_datetime(dt: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(dt, DATETIME_FMT).ok()
}

/// Formats hour/minute as "HH:MM" or "H:MM am/pm".
pub fn fmt_time(hour: u32, minute: u32, use_12h: bool) -> String {
    if !use_12h {
        return format!("{:02}:{:02}", hour, minute);
    }

    let (hour_12, period) = match hour {
        0 => (12, "am"),
        1..=11 => (hour, "am"),
        12 => (12, "pm"),
        _ => (hour - 12, "pm"),
    };

    format!("{}:{:02} {}", hour_12, minute, period)
}

/// Formats datetime as absolute display.
/// (e.g., "jan 15 10:00 am" or "jan 15" for all-day)
pub fn fmt_datetime(dt: NaiveDateTime, use_12h: bool) -> String {
    let is_all_day = dt.hour() == 0 && dt.minute() == 0;
    let date = dt.format("%b %-d").to_string().to_lowercase();

    if is_all_day {
        date
    } else {
        format!("{} {}", date, fmt_time(dt.hour(), dt.minute(), use_12h))
    }
}

/// Formats a date as a day group header.
/// (e.g., "today", "tomorrow", or "tuesday, jan 22")
pub fn fmt_day_header(event_date: NaiveDate, today: NaiveDate) -> String {
    let days_diff = (event_date - today).num_days();
    match days_diff {
        0 => "today".to_string(),
        1 => "tomorrow".to_string(),
        _ => event_date.format("%A, %b %-d").to_string().to_lowercase(),
    }
}

/// Formats event time for display within a day group.
/// All-day events return "all day". Today uses relative time, other days just the time.
pub fn fmt_time_in_group(
    event_dt: NaiveDateTime,
    now_dt: NaiveDateTime,
    is_today: bool,
    is_all_day: bool,
    use_12h: bool,
) -> String {
    if is_all_day {
        return "all day".to_string();
    }

    if is_today {
        fmt_relative_time(event_dt, now_dt, use_12h)
    } else {
        fmt_time(event_dt.hour(), event_dt.minute(), use_12h)
    }
}

/// Formats event time relative to now.
/// (e.g., "now", "in 30 min", "today 5 pm", "tmrw 9:00 am", or absolute)
/// Note: Caller should handle all-day events before calling this function.
pub fn fmt_relative_time(event_dt: NaiveDateTime, now_dt: NaiveDateTime, use_12h: bool) -> String {
    let minutes = event_dt.signed_duration_since(now_dt).num_minutes();

    // Past events or >24h away: absolute format
    if !(0..=24 * 60).contains(&minutes) {
        return fmt_datetime(event_dt, use_12h);
    }

    let is_tomorrow = event_dt.date() != now_dt.date();

    match minutes {
        0 => "now".to_string(),
        1..=9 => format!("in {} min", minutes),
        10..=55 => format!("in {} min", ((minutes + 2) / 5) * 5),
        56..=299 => {
            let time = fmt_time(event_dt.hour(), event_dt.minute(), use_12h);
            let whole_hours = minutes / 60;
            let remainder = minutes % 60;

            // Show .5 if within 10 min of half hour (20-40 min past)
            let relative = if (20..=40).contains(&remainder) {
                format!("{}.5 hrs", whole_hours)
            } else {
                // Round to nearest hour (>40 min rounds up)
                let hours = if remainder > 40 {
                    whole_hours + 1
                } else {
                    whole_hours
                };
                if hours == 1 {
                    "1 hr".to_string()
                } else {
                    format!("{} hrs", hours)
                }
            };

            format!("{} ({})", time, relative)
        }
        _ if is_tomorrow => {
            let time = fmt_time(event_dt.hour(), event_dt.minute(), use_12h);
            format!("tmrw {}", time)
        }
        _ => {
            let time = fmt_time(event_dt.hour(), event_dt.minute(), use_12h);
            format!("today {}", time)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    const ICS_TIMED_EVENT: &str = indoc! {"
        BEGIN:VCALENDAR
        VERSION:2.0
        BEGIN:VEVENT
        DTSTART:20240115T100000
        DTEND:20240115T110000
        SUMMARY:Team Standup
        LOCATION:https://zoom.us/j/123
        END:VEVENT
        END:VCALENDAR
    "};

    const ICS_ALL_DAY_EVENT: &str = indoc! {"
        BEGIN:VCALENDAR
        VERSION:2.0
        BEGIN:VEVENT
        DTSTART;VALUE=DATE:20240115
        DTEND;VALUE=DATE:20240116
        SUMMARY:Company Holiday
        END:VEVENT
        END:VCALENDAR
    "};

    const ICS_UTC_EVENT: &str = indoc! {"
        BEGIN:VCALENDAR
        VERSION:2.0
        BEGIN:VEVENT
        DTSTART:20240115T150000Z
        DTEND:20240115T160000Z
        SUMMARY:UTC Meeting
        END:VEVENT
        END:VCALENDAR
    "};

    const ICS_MULTIPLE_EVENTS: &str = indoc! {"
        BEGIN:VCALENDAR
        VERSION:2.0
        BEGIN:VEVENT
        DTSTART:20240115T100000
        SUMMARY:First Event
        END:VEVENT
        BEGIN:VEVENT
        DTSTART:20240115T140000
        SUMMARY:Second Event
        END:VEVENT
        END:VCALENDAR
    "};

    fn fmt(event: &str, now: &str) -> String {
        let event_dt = parse_datetime(event).unwrap();
        let now_dt = parse_datetime(now).unwrap();
        fmt_relative_time(event_dt, now_dt, true)
    }

    #[test]
    fn test_parse_timed_event() {
        let events = parse_ics(ICS_TIMED_EVENT.as_bytes(), 0).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].summary, "Team Standup");
        assert_eq!(events[0].start.hour(), 10);
        assert_eq!(events[0].start.minute(), 0);
        assert_eq!(
            events[0].location,
            Some("https://zoom.us/j/123".to_string())
        );
        assert!(events[0].is_video_call());
    }

    #[test]
    fn test_parse_all_day_event() {
        let events = parse_ics(ICS_ALL_DAY_EVENT.as_bytes(), 0).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].summary, "Company Holiday");
        // All-day events should have 00:00 time
        assert_eq!(events[0].start.hour(), 0);
        assert_eq!(events[0].start.minute(), 0);
    }

    #[test]
    fn test_parse_utc_event() {
        // With offset 0, UTC time stays as-is (15:00 UTC -> 15:00)
        let events = parse_ics(ICS_UTC_EVENT.as_bytes(), 0).unwrap();
        assert_eq!(events[0].start.hour(), 15);

        // With EST offset (-300 min), UTC time is converted (15:00 UTC -> 10:00 EST)
        let events = parse_ics(ICS_UTC_EVENT.as_bytes(), -300).unwrap();
        assert_eq!(events[0].start.hour(), 10);
    }

    #[test]
    fn test_parse_multiple_events() {
        let events = parse_ics(ICS_MULTIPLE_EVENTS.as_bytes(), 0).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].summary, "First Event");
        assert_eq!(events[1].summary, "Second Event");
    }

    #[test]
    fn test_video_call_detection() {
        let zoom = Event {
            summary: "Call".into(),
            start: NaiveDateTime::default(),
            end: None,
            location: Some("https://zoom.us/j/123".into()),
            is_all_day: false,
        };
        let meet = Event {
            summary: "Call".into(),
            start: NaiveDateTime::default(),
            end: None,
            location: Some("https://meet.google.com/abc".into()),
            is_all_day: false,
        };
        let teams = Event {
            summary: "Call".into(),
            start: NaiveDateTime::default(),
            end: None,
            location: Some("https://teams.microsoft.com/l/meetup".into()),
            is_all_day: false,
        };
        let office = Event {
            summary: "Meeting".into(),
            start: NaiveDateTime::default(),
            end: None,
            location: Some("Conference Room A".into()),
            is_all_day: false,
        };
        let none = Event {
            summary: "Meeting".into(),
            start: NaiveDateTime::default(),
            end: None,
            location: None,
            is_all_day: false,
        };

        assert!(zoom.is_video_call());
        assert!(meet.is_video_call());
        assert!(teams.is_video_call());
        assert!(!office.is_video_call());
        assert!(!none.is_video_call());
    }

    #[test]
    fn test_is_in_progress() {
        let event = Event {
            summary: "Meeting".into(),
            start: parse_datetime("2024-01-15 10:00").unwrap(),
            end: parse_datetime("2024-01-15 11:00"),
            location: None,
            is_all_day: false,
        };

        // Before start
        assert!(!event.is_in_progress(parse_datetime("2024-01-15 09:59").unwrap()));
        // At start
        assert!(event.is_in_progress(parse_datetime("2024-01-15 10:00").unwrap()));
        // During
        assert!(event.is_in_progress(parse_datetime("2024-01-15 10:30").unwrap()));
        // At end (no longer in progress)
        assert!(!event.is_in_progress(parse_datetime("2024-01-15 11:00").unwrap()));
        // After end
        assert!(!event.is_in_progress(parse_datetime("2024-01-15 11:01").unwrap()));

        // Event without end time is never "in progress"
        let no_end = Event {
            summary: "No End".into(),
            start: parse_datetime("2024-01-15 10:00").unwrap(),
            end: None,
            location: None,
            is_all_day: false,
        };
        assert!(!no_end.is_in_progress(parse_datetime("2024-01-15 10:30").unwrap()));

        // Event with non-zero seconds in start time - should NOT show as in-progress before it starts
        let event_with_secs = Event {
            summary: "Meeting".into(),
            start: NaiveDate::from_ymd_opt(2024, 1, 15)
                .unwrap()
                .and_hms_opt(10, 0, 30)
                .unwrap(), // 10:00:30
            end: Some(
                NaiveDate::from_ymd_opt(2024, 1, 15)
                    .unwrap()
                    .and_hms_opt(11, 0, 0)
                    .unwrap(),
            ),
            location: None,
            is_all_day: false,
        };
        // At 10:00:15, event hasn't started yet (starts at 10:00:30)
        let now_before = NaiveDate::from_ymd_opt(2024, 1, 15)
            .unwrap()
            .and_hms_opt(10, 0, 15)
            .unwrap();
        assert!(!event_with_secs.is_in_progress(now_before));
    }

    #[test]
    fn test_is_active_on() {
        // Multi-day all-day event: Jan 15-18 (3 days)
        let multi_day = Event {
            summary: "Conference".into(),
            start: parse_datetime("2024-01-15 00:00").unwrap(),
            end: parse_datetime("2024-01-18 00:00"),
            location: None,
            is_all_day: true,
        };
        assert!(!multi_day.is_active_on(NaiveDate::from_ymd_opt(2024, 1, 14).unwrap()));
        assert!(multi_day.is_active_on(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()));
        assert!(multi_day.is_active_on(NaiveDate::from_ymd_opt(2024, 1, 16).unwrap()));
        assert!(multi_day.is_active_on(NaiveDate::from_ymd_opt(2024, 1, 17).unwrap()));
        assert!(!multi_day.is_active_on(NaiveDate::from_ymd_opt(2024, 1, 18).unwrap())); // end is exclusive

        // Single-day all-day event
        let single_day = Event {
            summary: "Holiday".into(),
            start: parse_datetime("2024-01-15 00:00").unwrap(),
            end: parse_datetime("2024-01-16 00:00"),
            location: None,
            is_all_day: true,
        };
        assert!(!single_day.is_active_on(NaiveDate::from_ymd_opt(2024, 1, 14).unwrap()));
        assert!(single_day.is_active_on(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()));
        assert!(!single_day.is_active_on(NaiveDate::from_ymd_opt(2024, 1, 16).unwrap()));

        // Timed event - only active on start date
        let timed = Event {
            summary: "Meeting".into(),
            start: parse_datetime("2024-01-15 10:00").unwrap(),
            end: parse_datetime("2024-01-15 11:00"),
            location: None,
            is_all_day: false,
        };
        assert!(!timed.is_active_on(NaiveDate::from_ymd_opt(2024, 1, 14).unwrap()));
        assert!(timed.is_active_on(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()));
        assert!(!timed.is_active_on(NaiveDate::from_ymd_opt(2024, 1, 16).unwrap()));

        // Timed event spanning midnight (11pm - 1am) - should be active on both days
        let overnight = Event {
            summary: "Overnight".into(),
            start: parse_datetime("2024-01-15 23:00").unwrap(),
            end: parse_datetime("2024-01-16 01:00"),
            location: None,
            is_all_day: false,
        };
        assert!(!overnight.is_active_on(NaiveDate::from_ymd_opt(2024, 1, 14).unwrap()));
        assert!(overnight.is_active_on(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()));
        assert!(overnight.is_active_on(NaiveDate::from_ymd_opt(2024, 1, 16).unwrap())); // spans into this day
        assert!(!overnight.is_active_on(NaiveDate::from_ymd_opt(2024, 1, 17).unwrap()));
    }

    #[test]
    fn test_now() {
        assert_eq!(fmt("2024-01-15 10:00", "2024-01-15 10:00"), "now");
    }

    #[test]
    fn test_minutes() {
        // < 10 min: exact
        assert_eq!(fmt("2024-01-15 10:05", "2024-01-15 10:00"), "in 5 min");
        assert_eq!(fmt("2024-01-15 10:09", "2024-01-15 10:00"), "in 9 min");
        // >= 10 min: rounded to nearest 5
        assert_eq!(fmt("2024-01-15 10:10", "2024-01-15 10:00"), "in 10 min");
        assert_eq!(fmt("2024-01-15 10:12", "2024-01-15 10:00"), "in 10 min");
        assert_eq!(fmt("2024-01-15 10:13", "2024-01-15 10:00"), "in 15 min");
        assert_eq!(fmt("2024-01-15 10:30", "2024-01-15 10:00"), "in 30 min");
        assert_eq!(fmt("2024-01-15 10:55", "2024-01-15 10:00"), "in 55 min");
        assert_eq!(
            fmt("2024-01-15 10:56", "2024-01-15 10:00"),
            "10:56 am (1 hr)"
        );
    }

    #[test]
    fn test_hours_round_down() {
        // 0-19 min past hour rounds down
        assert_eq!(
            fmt("2024-01-15 11:00", "2024-01-15 10:00"),
            "11:00 am (1 hr)"
        );
        assert_eq!(
            fmt("2024-01-15 11:15", "2024-01-15 10:00"),
            "11:15 am (1 hr)"
        );
        assert_eq!(
            fmt("2024-01-15 11:19", "2024-01-15 10:00"),
            "11:19 am (1 hr)"
        );
        assert_eq!(
            fmt("2024-01-15 12:10", "2024-01-15 10:00"),
            "12:10 pm (2 hrs)"
        );
    }

    #[test]
    fn test_hours_half() {
        // 20-40 min past hour shows .5
        assert_eq!(
            fmt("2024-01-15 11:20", "2024-01-15 10:00"),
            "11:20 am (1.5 hrs)"
        );
        assert_eq!(
            fmt("2024-01-15 11:30", "2024-01-15 10:00"),
            "11:30 am (1.5 hrs)"
        );
        assert_eq!(
            fmt("2024-01-15 11:40", "2024-01-15 10:00"),
            "11:40 am (1.5 hrs)"
        );
        assert_eq!(
            fmt("2024-01-15 12:30", "2024-01-15 10:00"),
            "12:30 pm (2.5 hrs)"
        );
    }

    #[test]
    fn test_hours_round_up() {
        // 41-59 min past hour rounds up
        assert_eq!(
            fmt("2024-01-15 11:45", "2024-01-15 10:00"),
            "11:45 am (2 hrs)"
        );
        assert_eq!(
            fmt("2024-01-15 11:55", "2024-01-15 10:00"),
            "11:55 am (2 hrs)"
        );
        assert_eq!(
            fmt("2024-01-15 12:50", "2024-01-15 10:00"),
            "12:50 pm (3 hrs)"
        );
    }

    #[test]
    fn test_today() {
        assert_eq!(fmt("2024-01-15 18:00", "2024-01-15 10:00"), "today 6:00 pm");
        assert_eq!(fmt("2024-01-15 18:30", "2024-01-15 10:00"), "today 6:30 pm");
    }

    #[test]
    fn test_tomorrow() {
        assert_eq!(fmt("2024-01-16 09:00", "2024-01-15 20:00"), "tmrw 9:00 am");
        assert_eq!(fmt("2024-01-16 14:30", "2024-01-15 20:00"), "tmrw 2:30 pm");
    }

    #[test]
    fn test_all_day_events() {
        // All-day events get "all day" label via fmt_time_in_group
        let event_dt = parse_datetime("2024-01-15 00:00").unwrap();
        let now_dt = parse_datetime("2024-01-15 10:00").unwrap();
        assert_eq!(
            fmt_time_in_group(event_dt, now_dt, true, true, true),
            "all day"
        );
        assert_eq!(
            fmt_time_in_group(event_dt, now_dt, false, true, true),
            "all day"
        );
    }

    #[test]
    fn test_beyond_24h() {
        // Events >24h away get absolute format
        assert_eq!(
            fmt("2024-01-17 10:00", "2024-01-15 10:00"),
            "jan 17 10:00 am"
        );
    }

    #[test]
    fn test_past_events() {
        // Past events get absolute format
        assert_eq!(
            fmt("2024-01-15 08:00", "2024-01-15 10:00"),
            "jan 15 8:00 am"
        );
    }

    #[test]
    fn test_filter_future_keeps_in_progress() {
        let now = parse_datetime("2024-01-15 10:30").unwrap();

        let events = vec![
            // In progress: started 10:00, ends 11:00
            Event {
                summary: "In Progress".into(),
                start: parse_datetime("2024-01-15 10:00").unwrap(),
                end: parse_datetime("2024-01-15 11:00"),
                location: None,
                is_all_day: false,
            },
            // Fully past: started 08:00, ended 09:00
            Event {
                summary: "Already Ended".into(),
                start: parse_datetime("2024-01-15 08:00").unwrap(),
                end: parse_datetime("2024-01-15 09:00"),
                location: None,
                is_all_day: false,
            },
            // Future: starts 14:00
            Event {
                summary: "Future".into(),
                start: parse_datetime("2024-01-15 14:00").unwrap(),
                end: parse_datetime("2024-01-15 15:00"),
                location: None,
                is_all_day: false,
            },
            // Past with no end time: started 08:00
            Event {
                summary: "Past No End".into(),
                start: parse_datetime("2024-01-15 08:00").unwrap(),
                end: None,
                location: None,
                is_all_day: false,
            },
        ];

        let filtered = filter_future(events, Some(now), 10);
        let summaries: Vec<&str> = filtered.iter().map(|e| e.summary.as_str()).collect();

        assert_eq!(summaries, vec!["In Progress", "Future"]);
    }
}
