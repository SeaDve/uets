use adw::{prelude::*, subclass::prelude::*};
use gtk::{
    gio,
    glib::{self, clone},
};

use crate::{detector::Detector, tracker::Tracker, ui::Window, APP_ID};

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Application {
        pub(super) detector: Detector,
        pub(super) tracker: Tracker,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Application {
        const NAME: &'static str = "UetsApplication";
        type Type = super::Application;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for Application {}

    impl ApplicationImpl for Application {
        fn activate(&self) {
            self.parent_activate();

            let obj = self.obj();

            obj.window().present();
        }

        fn startup(&self) {
            self.parent_startup();

            let obj = self.obj();

            self.detector.connect_detected(clone!(
                #[weak]
                obj,
                move |_, id| {
                    let imp = obj.imp();
                    imp.tracker.handle_entity(id);
                }
            ));
        }
    }

    impl GtkApplicationImpl for Application {}
    impl AdwApplicationImpl for Application {}
}

glib::wrapper! {
    pub struct Application(ObjectSubclass<imp::Application>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl Application {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("application-id", APP_ID)
            .build()
    }

    pub fn get() -> Self {
        debug_assert!(
            gtk::is_initialized_main_thread(),
            "application must only be accessed in the main thread"
        );

        gio::Application::default().unwrap().downcast().unwrap()
    }

    pub fn detector(&self) -> &Detector {
        &self.imp().detector
    }

    pub fn tracker(&self) -> &Tracker {
        &self.imp().tracker
    }

    fn window(&self) -> Window {
        self.active_window()
            .map_or_else(|| Window::new(self), |w| w.downcast().unwrap())
    }
}
