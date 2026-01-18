#!/usr/bin/env bash
# Generates a test ICS file with events at various time offsets from now.
# Usage: ./scripts/gen-test-ics.sh > /tmp/test.ics

set -euo pipefail

# Current epoch timestamp 
now=$(date +%s)

# Helper to generate a VEVENT
# Args: offset_minutes, summary, [location]
event() {
    local offset_min=$1
    local summary=$2
    local location=${3:-}

    local start_ts=$((now + offset_min * 60))
    local end_ts=$((start_ts + 3600))  # 1 hr

    local start=$(date -d "@$start_ts" +%Y%m%dT%H%M%S)
    local end=$(date -d "@$end_ts" +%Y%m%dT%H%M%S)
    local uid="test-${offset_min}-$$@zj-cal"

    echo "BEGIN:VEVENT"
    echo "UID:$uid"
    echo "DTSTART:$start"
    echo "DTEND:$end"
    echo "SUMMARY:$summary"
    [[ -n "$location" ]] && echo "LOCATION:$location"
    echo "END:VEVENT"
}

cat <<'HEADER'
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//zj-cal//Test Calendar//EN
HEADER

event 0      "Team Standup"
event 5      "Quick Sync"
event 15     "Code Review"
event 30     "1:1"
event 45     "Sprint Planning"
event 60     "Design Review"
event 90     "Backlog Grooming"
event 120    "Tech Debt Discussion"
event 180    "Platform Team Sync"
event 240    "Architecture Review"
event 360    "Product Demo"
event 480    "All Hands Meeting"
event 720    "Quarterly Planning"
event 1080   "Customer Call"                    "https://zoom.us/j/123"
event 1440   "Board Meeting"                    "https://meet.google.com/abc"
event 2880   "Conference Prep"
event 4320   "Offsite Planning"                 "https://teams.microsoft.com/l/meetup"

echo "END:VCALENDAR"
