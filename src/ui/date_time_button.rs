use chrono::{NaiveTime, Utc};
use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};

use crate::{date_time, date_time_boxed::DateTimeBoxed, ui::date_time_picker::DateTimePicker};

const DEFAULT_SHOW_TIME: bool = true;

mod imp {
    use std::{cell::Cell, marker::PhantomData};

    use super::*;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::DateTimeButton)]
    #[template(resource = "/io/github/seadve/Uets/ui/date_time_button.ui")]
    pub struct DateTimeButton {
        #[property(get = Self::dt, set = Self::set_dt, explicit_notify, nullable)]
        pub(super) dt: PhantomData<Option<DateTimeBoxed>>,
        #[property(get, set = Self::set_show_time, explicit_notify, default = DEFAULT_SHOW_TIME)]
        pub(super) show_time: Cell<bool>,

        #[template_child]
        pub(super) button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) dt_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) dt_picker: TemplateChild<DateTimePicker>,

        pub(super) dt_inner: Cell<Option<DateTimeBoxed>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DateTimeButton {
        const NAME: &'static str = "UetsDateTimeButton";
        type Type = super::DateTimeButton;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("date-time-button.clear", None, move |obj, _, _| {
                let imp = obj.imp();

                obj.set_dt(None::<DateTimeBoxed>);

                imp.button.popdown();
            });
            klass.install_action("date-time-button.done", None, move |obj, _, _| {
                let imp = obj.imp();

                obj.set_dt(Some(DateTimeBoxed(imp.dt_picker.dt_utc())));

                imp.button.popdown();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for DateTimeButton {
        fn constructed(&self) {
            self.parent_constructed();

            self.show_time.set(DEFAULT_SHOW_TIME);

            let obj = self.obj();

            self.button.set_create_popup_func(clone!(
                #[weak]
                obj,
                move |_| {
                    let imp = obj.imp();

                    if let Some(dt) = obj.dt() {
                        imp.dt_picker.set_dt_utc(dt.0);
                    } else {
                        imp.dt_picker.set_dt_utc(Utc::now());
                    }
                }
            ));

            self.dt_picker.connect_dt_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_dt_label();
                }
            ));

            obj.update_label();
            obj.update_dt_label();
            obj.update_dt_picker_show_time();
        }

        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for DateTimeButton {}

    impl DateTimeButton {
        fn dt(&self) -> Option<DateTimeBoxed> {
            if self.show_time.get() {
                self.dt_inner.get()
            } else {
                self.dt_inner
                    .get()
                    .map(|dt| DateTimeBoxed(dt.0.with_time(NaiveTime::MIN).single().unwrap()))
            }
        }

        fn set_dt(&self, dt: Option<DateTimeBoxed>) {
            let obj = self.obj();

            if dt == obj.dt() {
                return;
            }

            self.dt_inner.set(dt);
            obj.update_label();
            obj.notify_dt();
        }

        fn set_show_time(&self, show_time: bool) {
            let obj = self.obj();

            if show_time == self.show_time.get() {
                return;
            }

            self.show_time.set(show_time);

            obj.update_label();
            obj.update_dt_label();
            obj.update_dt_picker_show_time();

            obj.notify_dt();
            obj.notify_show_time();
        }
    }
}

glib::wrapper! {
    pub struct DateTimeButton(ObjectSubclass<imp::DateTimeButton>)
        @extends gtk::Widget;
}

impl DateTimeButton {
    pub fn new() -> Self {
        glib::Object::new()
    }

    fn update_label(&self) {
        let imp = self.imp();

        if let Some(dt) = self.dt() {
            let label = if self.show_time() {
                date_time::format::human_readable(dt.0)
            } else {
                date_time::format::human_readable_date(dt.0)
            };
            imp.label.set_label(&label);
        } else {
            imp.label.set_label("None");
        }
    }

    fn update_dt_label(&self) {
        let imp = self.imp();

        let dt_unboxed = imp.dt_picker.dt_utc();
        let label = if self.show_time() {
            date_time::format::human_readable(dt_unboxed)
        } else {
            date_time::format::human_readable_date(dt_unboxed)
        };
        imp.dt_label.set_label(&label);
    }

    fn update_dt_picker_show_time(&self) {
        let imp = self.imp();

        let show_time = self.show_time();
        imp.dt_picker.set_show_time(show_time);
    }
}
