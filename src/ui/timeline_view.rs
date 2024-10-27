use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::{timeline::Timeline, ui::timeline_row::TimelineRow};

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/timeline_view.ui")]
    pub struct TimelineView {
        #[template_child]
        pub(super) scrolled_window: TemplateChild<gtk::ScrolledWindow>, // Unused
        #[template_child]
        pub(super) list_view: TemplateChild<gtk::ListView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TimelineView {
        const NAME: &'static str = "UetsTimelineView";
        type Type = super::TimelineView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            TimelineRow::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TimelineView {
        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for TimelineView {}
}

glib::wrapper! {
    pub struct TimelineView(ObjectSubclass<imp::TimelineView>)
        @extends gtk::Widget;
}

impl TimelineView {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn bind_timeline(&self, timeline: &Timeline) {
        let imp = self.imp();

        let selection_model = gtk::NoSelection::new(Some(timeline.clone()));
        imp.list_view.set_model(Some(&selection_model));
    }
}
