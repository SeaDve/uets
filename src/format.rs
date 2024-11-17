use chrono::{DateTime, Datelike, Local, TimeDelta, Utc};
use gtk::glib;

pub fn transfer_progress(sent_bytes: u64, total_bytes: u64) -> String {
    format!(
        "{} / {}",
        glib::format_size(sent_bytes),
        glib::format_size(total_bytes)
    )
}

pub fn fuzzy_dt(dt: DateTime<Utc>) -> String {
    let dt = dt.with_timezone(&Local);
    let now = Local::now();

    if dt.year() == now.year() && dt.month() == now.month() && dt.day() == now.day() {
        dt.format("today at %r").to_string() // today at 13:21:16 AM
    } else if (now - dt).num_hours() <= 30 {
        dt.format("yesterday at %r").to_string() // yesterday at 13:21:16 AM
    } else {
        dt.format("%a, %-d %b %Y at %r").to_string() // Sun, 3 Nov 2024 at 13:21:16 AM
    }
}

/// Formats time as duration.
pub fn duration(time_span: TimeDelta) -> String {
    let secs = time_span.num_seconds();

    let days_display = secs / 86400;
    let hours_display = secs / 3600;
    let minutes_display = (secs % 3600) / 60;
    let seconds_display = secs % 60;

    let days_display_str = format!(
        "{} {}",
        days_display,
        pluralize("day", "days", days_display as u32)
    );
    let hours_display_str = format!(
        "{} {}",
        hours_display,
        pluralize("hour", "hours", hours_display as u32)
    );
    let minutes_display_str = format!(
        "{} {}",
        minutes_display,
        pluralize("minute", "minutes", minutes_display as u32)
    );
    let seconds_display_str = format!(
        "{} {}",
        seconds_display,
        pluralize("second", "seconds", seconds_display as u32)
    );

    if days_display > 0 {
        // 3 days 4 hours 5 minutes 6 seconds
        format!(
            "{} {} {} {}",
            days_display_str, hours_display_str, minutes_display_str, seconds_display_str
        )
    } else if hours_display > 0 {
        // 4 hours 5 minutes 6 seconds
        format!(
            "{} {} {}",
            hours_display_str, minutes_display_str, seconds_display_str
        )
    } else if minutes_display > 0 {
        // 5 minutes 6 seconds
        format!("{} {}", minutes_display_str, seconds_display_str)
    } else {
        // 6 seconds
        seconds_display_str
    }
}

fn pluralize<'a>(singular: &'a str, plural: &'a str, n: u32) -> &'a str {
    match n {
        0 => plural,
        1 => singular,
        2.. => plural,
    }
}
