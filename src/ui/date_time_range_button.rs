use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::{date_time_range::DateTimeRange, ui::date_time_range_dialog::DateTimeRangeDialog};

mod imp {
    use std::cell::Cell;

    use super::*;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::DateTimeRangeButton)]
    #[template(resource = "/io/github/seadve/Uets/ui/date_time_range_button.ui")]
    pub struct DateTimeRangeButton {
        #[property(get, set = Self::set_range, explicit_notify)]
        pub(super) range: Cell<DateTimeRange>,

        #[template_child]
        pub(super) button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DateTimeRangeButton {
        const NAME: &'static str = "UetsDateTimeRangeButton";
        type Type = super::DateTimeRangeButton;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action_async(
                "date-time-range-button.pick",
                None,
                |obj, _, _| async move {
                    let range = obj.range();
                    let initial_range = if range.is_all_time() {
                        DateTimeRange::today()
                    } else {
                        range
                    };

                    if let Ok(new_range) =
                        DateTimeRangeDialog::pick(initial_range, Some(&obj)).await
                    {
                        obj.set_range(new_range);
                    }
                },
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for DateTimeRangeButton {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.update_label();
        }

        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for DateTimeRangeButton {}

    impl DateTimeRangeButton {
        fn set_range(&self, range: DateTimeRange) {
            let obj = self.obj();

            if range == obj.range() {
                return;
            }

            self.range.set(range);
            obj.update_label();
            obj.notify_range();
        }
    }
}

glib::wrapper! {
    pub struct DateTimeRangeButton(ObjectSubclass<imp::DateTimeRangeButton>)
        @extends gtk::Widget;
}

impl DateTimeRangeButton {
    pub fn new() -> Self {
        glib::Object::new()
    }

    fn update_label(&self) {
        let imp = self.imp();

        let range = self.range();
        imp.label.set_label(&range.short_label_markup());
    }
}
