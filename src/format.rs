use gtk::glib;

/// Formats time as duration.
pub fn duration(time_span: glib::TimeSpan) -> String {
    let secs = time_span.as_seconds();

    let days_display = secs / 86400;
    let hours_display = secs / 3600;
    let minutes_display = (secs % 3600) / 60;
    let seconds_display = secs % 60;

    let days_display_str = n_f(
        "{time} day",
        "{time} days",
        days_display as u32,
        &[("time", &days_display.to_string())],
    );
    let hours_display_str = n_f(
        "{time} hour",
        "{time} hours",
        hours_display as u32,
        &[("time", &hours_display.to_string())],
    );
    let minutes_display_str = n_f(
        "{time} minute",
        "{time} minutes",
        minutes_display as u32,
        &[("time", &minutes_display.to_string())],
    );
    let seconds_display_str = n_f(
        "{time} second",
        "{time} seconds",
        seconds_display as u32,
        &[("time", &seconds_display.to_string())],
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

fn n_f(singular: &str, plural: &str, n: u32, args: &[(&str, &str)]) -> String {
    let s = match n {
        0 => plural,
        1 => singular,
        2.. => plural,
    };
    freplace(s.to_string(), args)
}

/// Replace variables in the given string using the given key-value tuples.
///
/// The expected format to replace is `{name}`, where `name` is the first string
/// in a key-value tuple.
fn freplace(s: String, args: &[(&str, &str)]) -> String {
    // This function is useless if there are no arguments
    debug_assert!(!args.is_empty(), "atleast one key-value pair must be given");

    // We could check here if all keys were used, but some translations might
    // not use all variables, so we don't do that.

    let mut s = s;
    for (key, val) in args {
        s = s.replace(&format!("{{{key}}}"), val);
    }

    debug_assert!(!s.contains('{'), "all format variables must be replaced");

    if tracing::enabled!(tracing::Level::WARN) && s.contains('{') {
        tracing::warn!(
            "all format variables must be replaced, but some were not: {}",
            s
        );
    }

    s
}
