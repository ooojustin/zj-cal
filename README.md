# zj-cal

A Zellij plugin that displays upcoming calendar events from an ICS feed.

## Installation

Add to your Zellij config (`~/.config/zellij/config.kdl`):

```kdl
plugins {
    calendar location="https://github.com/ooojustin/zj-cal/releases/latest/download/zj-cal.wasm"
}
```

Then bind a key to launch it:

```kdl
keybinds {
    normal {
        bind "Ctrl a" {
            LaunchOrFocusPlugin "calendar" {
                floating true
                move_to_focused_tab true
            };
        }
    }
}
```

## Configuration

```kdl
calendar location="https://github.com/ooojustin/zj-cal/releases/latest/download/zj-cal.wasm" {
    ics_url "https://calendar.google.com/calendar/ical/.../basic.ics"
    refresh_interval "300"  // seconds (default: 300)
    time_format "24"        // "24" for 24-hour, "12" for 12-hour (default: 12-hour)
}
```

If `ics_url` is not set, the plugin will automatically use the `ZJ_CAL_ICS_URL` environment variable:

```bash
export ZJ_CAL_ICS_URL="https://calendar.google.com/calendar/ical/.../basic.ics"
```
