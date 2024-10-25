use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::{
    timeline::Timeline,
    timeline_item::{TimelineItem, TimelineItemKind},
};

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/timeline_view.ui")]
    pub struct TimelineView {
        #[template_child]
        pub(super) list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TimelineView {
        const NAME: &'static str = "UetsTimelineView";
        type Type = super::TimelineView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
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

        imp.list_box.bind_model(Some(timeline), |item| {
            let item = item.downcast_ref::<TimelineItem>().unwrap();

            let text = match item.kind() {
                TimelineItemKind::Entry => {
                    format!(
                        "{} enters {}",
                        item.entity().id(),
                        item.dt().fuzzy_display()
                    )
                }
                TimelineItemKind::Exit => {
                    format!("{} exits {}", item.entity().id(), item.dt().fuzzy_display())
                }
            };

            gtk::Label::new(Some(&text)).upcast()
        });
    }
}
