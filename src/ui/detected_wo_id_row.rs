use gtk::{gdk, glib, prelude::*, subclass::prelude::*};

use crate::{date_time, detected_wo_id_item::DetectedWoIdItem, Application};

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::DetectedWoIdRow)]
    #[template(resource = "/io/github/seadve/Uets/ui/detected_wo_id_row.ui")]
    pub struct DetectedWoIdRow {
        #[property(get, set = Self::set_item, explicit_notify)]
        pub(super) item: RefCell<Option<DetectedWoIdItem>>,

        #[template_child]
        pub(super) vbox: TemplateChild<gtk::Box>, // Unused
        #[template_child]
        pub(super) dt_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) picture: TemplateChild<gtk::Picture>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DetectedWoIdRow {
        const NAME: &'static str = "UetsDetectedWoIdRow";
        type Type = super::DetectedWoIdRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("detected-wo-id-row.delete", None, |obj, _, _| {
                let imp = obj.imp();

                if let Some(item) = imp.item.borrow().as_ref() {
                    if let Err(err) = Application::get().detected_wo_id_list().remove(&item.dt()) {
                        tracing::error!("Failed to remove detected without ID item: {:?}", err);
                    }
                }
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for DetectedWoIdRow {
        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for DetectedWoIdRow {}

    impl DetectedWoIdRow {
        fn set_item(&self, item: Option<DetectedWoIdItem>) {
            let obj = self.obj();

            if item == obj.item() {
                return;
            }

            if let Some(item) = &item {
                self.dt_label.set_text(&date_time::format::fuzzy(item.dt()));
                self.picture
                    .set_paintable(item.image().as_ref().and_then(|i| {
                        i.texture()
                            .inspect_err(|err| {
                                tracing::warn!("Failed get image texture: {:?}", err)
                            })
                            .ok()
                    }));
            } else {
                self.dt_label.set_text("");
                self.picture.set_paintable(gdk::Paintable::NONE);
            }

            self.item.replace(item);
            obj.notify_item();
        }
    }
}

glib::wrapper! {
    pub struct DetectedWoIdRow(ObjectSubclass<imp::DetectedWoIdRow>)
        @extends gtk::Widget;
}

impl DetectedWoIdRow {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
