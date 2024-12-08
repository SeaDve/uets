use std::rc::Rc;

use gtk::glib::clone;

use crate::{format, settings::Settings, ui::InformationRow};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LimitReached {
    Upper,
    Lower,
}

pub trait LabelExt {
    fn set_label_from_limit_reached(&self, count: u32, settings: &Settings);
}

impl LabelExt for gtk::Label {
    fn set_label_from_limit_reached(&self, count: u32, settings: &Settings) {
        if settings.compute_limit_reached(count).is_some() {
            self.set_use_markup(true);
            self.set_label(&format::red_markup(&count.to_string()))
        } else {
            self.set_use_markup(false);
            self.set_label(&count.to_string());
        }
    }
}

pub trait InformationRowExt {
    fn set_value_from_limit_reached(&self, count: u32, settings: &Settings);
}

impl InformationRowExt for InformationRow {
    fn set_value_from_limit_reached(&self, count: u32, settings: &Settings) {
        if settings.compute_limit_reached(count).is_some() {
            self.set_value_use_markup(true);
            self.set_value(format::red_markup(&count.to_string()))
        } else {
            self.set_value_use_markup(false);
            self.set_value(count.to_string());
        }
    }
}

pub trait SettingsExt {
    fn compute_limit_reached(&self, count: u32) -> Option<LimitReached>;
    fn connect_limit_reached_changed(&self, f: impl Fn(&Self) + 'static);
}

impl SettingsExt for Settings {
    fn compute_limit_reached(&self, count: u32) -> Option<LimitReached> {
        let lower = self.lower_limit_reached_alert_threshold();
        let upper = self.upper_limit_reached_alert_threshold();

        if lower >= upper {
            tracing::warn!("Lower >= upper limit");
            return None;
        }

        if count <= lower && self.enable_lower_limit_reached_alert() {
            Some(LimitReached::Lower)
        } else if count >= upper && self.enable_upper_limit_reached_alert() {
            Some(LimitReached::Upper)
        } else {
            None
        }
    }

    fn connect_limit_reached_changed(&self, f: impl Fn(&Self) + 'static) {
        let f = Rc::new(f);

        self.connect_lower_limit_reached_alert_threshold_changed(clone!(
            #[strong]
            f,
            move |s| f(s)
        ));

        self.connect_upper_limit_reached_alert_threshold_changed(clone!(
            #[strong]
            f,
            move |s| f(s)
        ));

        self.connect_enable_lower_limit_reached_alert_changed(clone!(
            #[strong]
            f,
            move |s| f(s)
        ));

        self.connect_enable_upper_limit_reached_alert_changed(clone!(
            #[strong]
            f,
            move |s| f(s)
        ));
    }
}
