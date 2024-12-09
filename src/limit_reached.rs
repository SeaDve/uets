use std::rc::Rc;

use gtk::glib::clone;

use crate::{
    format, settings::Settings, signal_handler_id_group::SignalHandlerIdGroup, ui::InformationRow,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LimitReached {
    Lower,
    Upper,
}

impl LimitReached {
    pub fn is_lower(&self) -> bool {
        matches!(self, LimitReached::Lower)
    }

    pub fn is_upper(&self) -> bool {
        matches!(self, LimitReached::Upper)
    }
}

pub trait LimitReachedLabelExt {
    fn set_label_from_limit_reached(&self, count: u32, settings: &Settings);
}

impl LimitReachedLabelExt for gtk::Label {
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

pub trait LimitReachedInformationRowExt {
    fn set_value_from_limit_reached(&self, count: u32, settings: &Settings);
}

impl LimitReachedInformationRowExt for InformationRow {
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

pub trait LimitReachedSettingsExt {
    fn compute_limit_reached(&self, count: u32) -> Option<LimitReached>;
    fn connect_limit_reached_threshold_changed(
        &self,
        f: impl Fn(&Self) + 'static,
    ) -> SignalHandlerIdGroup;
}

impl LimitReachedSettingsExt for Settings {
    fn compute_limit_reached(&self, count: u32) -> Option<LimitReached> {
        let lower = self.lower_limit_reached_threshold();
        let upper = self.upper_limit_reached_threshold();

        if lower >= upper {
            tracing::warn!("Lower >= upper limit");
            return None;
        }

        if count <= lower {
            Some(LimitReached::Lower)
        } else if count >= upper {
            Some(LimitReached::Upper)
        } else {
            None
        }
    }

    fn connect_limit_reached_threshold_changed(
        &self,
        f: impl Fn(&Self) + 'static,
    ) -> SignalHandlerIdGroup {
        let handler_ids = SignalHandlerIdGroup::new();

        let f = Rc::new(f);

        let handler_id = self.connect_lower_limit_reached_threshold_changed(clone!(
            #[strong]
            f,
            move |s| f(s)
        ));
        handler_ids.add(handler_id);

        let handler_id = self.connect_upper_limit_reached_threshold_changed(clone!(
            #[strong]
            f,
            move |s| f(s)
        ));
        handler_ids.add(handler_id);

        handler_ids
    }
}
