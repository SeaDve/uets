use chrono::TimeDelta;
use chrono_humanize::{Accuracy, HumanTime, Tense};
use gtk::glib;

pub fn red_markup(text: &str) -> String {
    format!(
        "<span foreground=\"red\">{}</span>",
        glib::markup_escape_text(text)
    )
}

pub fn transfer_progress(sent_bytes: u64, total_bytes: u64) -> String {
    format!(
        "{} / {}",
        glib::format_size(sent_bytes),
        glib::format_size(total_bytes)
    )
}

/// Formats time as duration.
pub fn duration(time_span: TimeDelta) -> String {
    let subsec_removed = TimeDelta::seconds(time_span.num_seconds());
    HumanTime::from(subsec_removed).to_text_en(Accuracy::Precise, Tense::Present)
}
