use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::timeline_item::{TimelineItem, TimelineItemKind};

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::TimelineRow)]
    #[template(resource = "/io/github/seadve/Uets/ui/timeline_row.ui")]
    pub struct TimelineRow {
        #[property(get, set = Self::set_item, explicit_notify)]
        pub(super) item: RefCell<Option<TimelineItem>>,

        #[template_child]
        pub(super) image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) dt_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) status_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TimelineRow {
        const NAME: &'static str = "UetsTimelineRow";
        type Type = super::TimelineRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for TimelineRow {
        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for TimelineRow {}

    impl TimelineRow {
        fn set_item(&self, item: Option<TimelineItem>) {
            let obj = self.obj();

            if item == obj.item() {
                return;
            }

            if let Some(ref item) = item {
                let dt_fuzzy_text = item.dt().to_local().fuzzy_display();
                self.dt_label.set_label(&dt_fuzzy_text);

                let id = item.entity().id();
                let text = match item.kind() {
                    TimelineItemKind::Entry => {
                        format!("<b>{}</b> enters", id)
                    }
                    TimelineItemKind::Exit => {
                        format!("<b>{}</b> exits", id)
                    }
                };
                self.status_label.set_label(&text);

                match item.kind() {
                    TimelineItemKind::Entry => {
                        self.image.set_icon_name(Some("arrow4-right-symbolic"));
                        self.image.remove_css_class("exit");
                        self.image.add_css_class("entry");
                    }
                    TimelineItemKind::Exit => {
                        self.image.set_icon_name(Some("arrow4-left-symbolic"));
                        self.image.remove_css_class("entry");
                        self.image.add_css_class("exit");
                    }
                };
            } else {
                self.dt_label.set_label("");
                self.status_label.set_label("");
            }

            self.item.replace(item);
            obj.notify_item();
        }
    }
}

glib::wrapper! {
    pub struct TimelineRow(ObjectSubclass<imp::TimelineRow>)
        @extends gtk::Widget;
}

impl TimelineRow {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
