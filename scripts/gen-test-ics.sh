#!/usr/bin/env bash
# Generates a test ICS file with events at various time offsets from now.
# Usage: ./scripts/gen-test-ics.sh > /tmp/test.ics

set -euo pipefail

# Current epoch timestamp 
now=$(date +%s)

# Generate a VEVENT
# Usage: event [-u] [-a] offset summary [location]
#   -u: Use UTC time (Z suffix)
#   -a: All-day event (offset in days)
event() {
    local date_flag="" suffix="" uid="test" mult=60 dur=3600 fmt="%Y%m%dT%H%M%S" vd=""

    [[ "${1:-}" == "-u" ]] && { date_flag="-u"; suffix="Z"; uid="utc"; shift; }

    [[ "${1:-}" == "-a" ]] && { mult=86400; dur=86400; fmt="%Y%m%d"; vd=";VALUE=DATE"; uid="allday"; suffix=""; shift; }

    local offset=$1 summary=$2 location=${3:-}
    local start_ts=$((now + offset * mult))
    local end_ts=$((start_ts + dur))
    local start=$(date $date_flag -d "@$start_ts" +$fmt)
    local end=$(date $date_flag -d "@$end_ts" +$fmt)

    echo "BEGIN:VEVENT"
    echo "UID:${uid}-${offset}-$$@zj-cal"
    echo "DTSTART${vd}:${start}${suffix}"
    echo "DTEND${vd}:${end}${suffix}"
    echo "SUMMARY:$summary"

    [[ -n "$location" ]] && echo "LOCATION:$location"

    echo "END:VEVENT"
}

cat <<'HEADER'
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//zj-cal//Test Calendar//EN
HEADER

event      0       "Team Standup"
event      5       "Quick Sync"
event -u   15      "Code Review (UTC)"
event      30      "1:1"
event      45      "Sprint Planning"
event      60      "Design Review"
event      90      "Backlog Grooming"
event      120     "Tech Debt Discussion"
event -u   180     "Platform Team Sync (UTC)"
event      240     "Architecture Review"
event      360     "Product Demo"
event      480     "All Hands Meeting"
event      720     "Quarterly Planning"
event -u   1080    "Customer Call (UTC)"            "https://zoom.us/j/123"
event      1440    "Board Meeting"                  "https://meet.google.com/abc"
event      2880    "Conference Prep"
event      4320    "Offsite Planning"               "https://teams.microsoft.com/l/meetup"

event -a   0       "Company Holiday"
event -a   1       "Team Offsite"
event -a   3       "Conference Day 1"

echo "END:VCALENDAR"
