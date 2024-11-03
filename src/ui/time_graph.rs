use chrono::Utc;
use gtk::{glib, prelude::*, subclass::prelude::*};
use plotters_cairo::CairoBackend;

use crate::time_graph;

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/time_graph.ui")]
    pub struct Graph {
        #[template_child]
        pub(super) no_data_revealer: TemplateChild<gtk::Revealer>,

        pub(super) data: RefCell<Vec<(chrono::DateTime<Utc>, u32)>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Graph {
        const NAME: &'static str = "UetsTimeGraph";
        type Type = super::TimeGraph;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Graph {
        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for Graph {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let obj = self.obj();

            let width = obj.width();
            let height = obj.height();

            if width == 0 || height == 0 {
                return;
            }

            let bounds = gtk::graphene::Rect::new(0.0, 0.0, width as f32, height as f32);
            let cr = snapshot.append_cairo(&bounds);
            let backend = CairoBackend::new(&cr, (width as u32, height as u32)).unwrap();

            if let Err(err) = time_graph::draw(backend, None, &self.data.borrow()) {
                tracing::error!("Failed to draw graph: {:?}", err);
            }

            self.parent_snapshot(snapshot);
        }
    }
}

glib::wrapper! {
    pub struct TimeGraph(ObjectSubclass<imp::Graph>)
        @extends gtk::Widget;
}

impl TimeGraph {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_data(&self, data: Vec<(chrono::DateTime<Utc>, u32)>) {
        let imp = self.imp();

        imp.data.replace(data);

        self.queue_draw();
        self.update_no_data_revealer();
    }

    fn update_no_data_revealer(&self) {
        let imp = self.imp();

        imp.no_data_revealer
            .set_reveal_child(imp.data.borrow().is_empty());
    }
}
