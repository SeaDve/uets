use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveDateTime, Utc};
use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};

use crate::ui::time_picker::{NaiveTimeBoxed, TimePicker};

const DEFAULT_SHOW_TIME: bool = true;

#[derive(Default, Clone, Copy, PartialEq, Eq, glib::Boxed)]
#[boxed_type(name = "UetsNaiveDateTimeBoxed")]
pub struct NaiveDateTimeBoxed(pub NaiveDateTime);

mod imp {
    use std::cell::Cell;

    use super::*;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::DateTimePicker)]
    #[template(resource = "/io/github/seadve/Uets/ui/date_time_picker.ui")]
    pub struct DateTimePicker {
        #[property(get, set = Self::set_dt, explicit_notify)]
        pub(super) dt: Cell<NaiveDateTimeBoxed>,
        #[property(get, set = Self::set_show_time, explicit_notify, default = DEFAULT_SHOW_TIME)]
        pub(super) show_time: Cell<bool>,

        #[template_child]
        pub(super) hbox: TemplateChild<gtk::Box>, // Unused
        #[template_child]
        pub(super) calendar: TemplateChild<gtk::Calendar>,
        #[template_child]
        pub(super) time_picker: TemplateChild<TimePicker>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DateTimePicker {
        const NAME: &'static str = "UetsDateTimePicker";
        type Type = super::DateTimePicker;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for DateTimePicker {
        fn constructed(&self) {
            self.parent_constructed();

            self.show_time.set(DEFAULT_SHOW_TIME);

            let obj = self.obj();

            self.calendar.connect_year_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_dt_from_ui();
                }
            ));
            self.calendar.connect_month_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_dt_from_ui();
                }
            ));
            self.calendar.connect_day_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_dt_from_ui();
                }
            ));
            self.time_picker.connect_time_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_dt_from_ui();
                }
            ));

            obj.update_ui_from_dt(NaiveDateTimeBoxed::default());
            obj.update_dt_from_ui();
            obj.update_time_picker_visibility();
        }

        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for DateTimePicker {}

    impl DateTimePicker {
        fn set_dt(&self, dt: NaiveDateTimeBoxed) {
            let obj = self.obj();

            if dt == self.dt.get() {
                return;
            }

            obj.update_ui_from_dt(dt);
            obj.update_dt_from_ui();
        }

        fn set_show_time(&self, show_time: bool) {
            let obj = self.obj();

            if show_time == self.show_time.get() {
                return;
            }

            self.show_time.set(show_time);
            obj.update_time_picker_visibility();
            obj.notify_show_time();
        }
    }
}

glib::wrapper! {
    pub struct DateTimePicker(ObjectSubclass<imp::DateTimePicker>)
        @extends gtk::Widget;
}

impl DateTimePicker {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_dt_utc(&self, dt: DateTime<Utc>) {
        let dt = NaiveDateTimeBoxed(dt.with_timezone(&Local).naive_local());
        self.set_dt(dt);
    }

    pub fn dt_utc(&self) -> DateTime<Utc> {
        let dt = self.dt();
        dt.0.and_local_timezone(Local).single().unwrap().to_utc()
    }

    pub fn mark_day(&self, day: u32) {
        let imp = self.imp();
        imp.calendar.mark_day(day);
    }

    fn update_ui_from_dt(&self, dt: NaiveDateTimeBoxed) {
        let imp = self.imp();

        let dt_unboxed = dt.0;
        imp.calendar.select_day(
            &glib::DateTime::new(
                &glib::TimeZone::local(),
                dt_unboxed.year(),
                dt_unboxed.month() as i32,
                dt_unboxed.day() as i32,
                0,
                0,
                0.0,
            )
            .unwrap(),
        );
        imp.time_picker.set_time(NaiveTimeBoxed(dt_unboxed.time()));
    }

    fn update_dt_from_ui(&self) {
        let imp = self.imp();

        let (year, month, day) = imp.calendar.date().ymd();
        let date = NaiveDate::from_ymd_opt(year, month as u32, day as u32).unwrap();
        let time = imp.time_picker.time().0;
        let dt = NaiveDateTimeBoxed(NaiveDateTime::new(date, time));

        if dt == imp.dt.get() {
            return;
        }

        imp.dt.set(dt);
        self.notify_dt()
    }

    fn update_time_picker_visibility(&self) {
        let imp = self.imp();

        let show_time = self.show_time();
        imp.time_picker.set_visible(show_time);
    }
}
