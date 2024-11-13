use chrono::{NaiveTime, Timelike};
use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};

#[derive(Default, Clone, Copy, PartialEq, Eq, glib::Boxed)]
#[boxed_type(name = "UetsNaiveTimeBoxed")]
pub struct NaiveTimeBoxed(pub NaiveTime);

mod imp {
    use std::cell::Cell;

    use super::*;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::TimePicker)]
    #[template(resource = "/io/github/seadve/Uets/ui/time_picker.ui")]
    pub struct TimePicker {
        #[property(get, set = Self::set_time, explicit_notify)]
        pub(super) time: Cell<NaiveTimeBoxed>,

        #[template_child]
        pub(super) hbox: TemplateChild<gtk::Box>, // Unused
        #[template_child]
        pub(super) hour_button: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub(super) minute_button: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub(super) second_button: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub(super) am_pm_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) am_pm_button_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TimePicker {
        const NAME: &'static str = "UetsTimePicker";
        type Type = super::TimePicker;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for TimePicker {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            self.hour_button.connect_output(clone!(
                #[weak]
                obj,
                #[upgrade_or_panic]
                move |button| obj.handle_button_output(button)
            ));
            self.hour_button.connect_value_changed(clone!(
                #[weak]
                obj,
                #[upgrade_or_panic]
                move |_| obj.update_time()
            ));

            self.minute_button.connect_output(clone!(
                #[weak]
                obj,
                #[upgrade_or_panic]
                move |button| obj.handle_button_output(button)
            ));
            self.minute_button.connect_value_changed(clone!(
                #[weak]
                obj,
                #[upgrade_or_panic]
                move |_| obj.update_time()
            ));

            self.second_button.connect_output(clone!(
                #[weak]
                obj,
                #[upgrade_or_panic]
                move |button| obj.handle_button_output(button)
            ));
            self.second_button.connect_value_changed(clone!(
                #[weak]
                obj,
                #[upgrade_or_panic]
                move |_| obj.update_time()
            ));

            self.am_pm_button.connect_clicked(clone!(
                #[weak]
                obj,
                #[upgrade_or_panic]
                move |_| {
                    obj.update_am_pm_button_label();
                    obj.update_time();
                }
            ));

            obj.update_am_pm_button_label();
        }

        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for TimePicker {}

    impl TimePicker {
        fn set_time(&self, time: NaiveTimeBoxed) {
            let obj = self.obj();

            if time == self.time.get() {
                return;
            }

            let time_unboxed = time.0;
            let (is_pm, hour12) = time_unboxed.hour12();
            let am_pm = if is_pm { AmPm::Pm } else { AmPm::Am };
            self.hour_button.set_value(hour12 as f64);
            self.minute_button.set_value(time_unboxed.minute() as f64);
            self.second_button.set_value(time_unboxed.second() as f64);
            self.am_pm_button_label.set_label(am_pm.as_str());

            obj.update_time();
        }
    }
}

glib::wrapper! {
    pub struct TimePicker(ObjectSubclass<imp::TimePicker>)
        @extends gtk::Widget;
}

impl TimePicker {
    pub fn new() -> Self {
        glib::Object::new()
    }

    fn handle_button_output(&self, button: &gtk::SpinButton) -> glib::Propagation {
        button.set_text(&format!("{:02}", button.value_as_int()));

        glib::Propagation::Stop
    }

    fn update_time(&self) {
        let imp = self.imp();

        let hour = {
            let mut ret = imp.hour_button.value_as_int();
            match AmPm::from_str(&imp.am_pm_button_label.label()) {
                AmPm::Am if ret == 12 => {
                    ret = 0;
                }
                AmPm::Pm if ret != 12 => {
                    ret += 12;
                }
                _ => {}
            }
            ret
        };
        let minute = imp.minute_button.value_as_int();
        let second = imp.second_button.value_as_int();
        let time = NaiveTimeBoxed(
            NaiveTime::from_hms_opt(hour as u32, minute as u32, second as u32).unwrap(),
        );

        if time == imp.time.get() {
            return;
        }

        imp.time.set(time);
        self.notify_time();
    }

    fn update_am_pm_button_label(&self) {
        let imp = self.imp();

        let am_pm = AmPm::from_str(&imp.am_pm_button_label.label());
        imp.am_pm_button_label.set_label(am_pm.rev().as_str());
    }
}

enum AmPm {
    Am,
    Pm,
}

impl AmPm {
    fn from_str(s: &str) -> Self {
        match s {
            "AM" => Self::Am,
            "PM" => Self::Pm,
            _ => unreachable!(),
        }
    }

    fn rev(&self) -> Self {
        match self {
            Self::Am => Self::Pm,
            Self::Pm => Self::Am,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Am => "AM",
            Self::Pm => "PM",
        }
    }
}
