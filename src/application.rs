use adw::{prelude::*, subclass::prelude::*};
use anyhow::Result;
use gtk::{
    gio,
    glib::{self, clone},
};

use crate::{db, detector::Detector, entity_tracker::EntityTracker, ui::Window, APP_ID};

mod imp {
    use std::cell::OnceCell;

    use super::*;

    #[derive(Default)]
    pub struct Application {
        pub(super) detector: Detector,
        pub(super) entity_tracker: OnceCell<EntityTracker>,
        pub(super) env: OnceCell<heed::Env>,
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

            tracing::info!("Starting up");

            let obj = self.obj();

            match init_env() {
                Ok((env, entity_tracker)) => {
                    self.env.set(env).unwrap();
                    self.entity_tracker.set(entity_tracker).unwrap();
                }
                Err(err) => {
                    tracing::debug!("Failed to init env: {:?}", err);
                    obj.quit();
                }
            }

            self.detector.connect_detected(clone!(
                #[weak]
                obj,
                move |_, id| {
                    if let Err(err) = obj.entity_tracker().handle_entity(id) {
                        tracing::error!("Failed to handle entity: {:?}", err);
                    }
                }
            ));
        }

        fn shutdown(&self) {
            if let Some(env) = self.env.get() {
                if let Err(err) = env.force_sync() {
                    tracing::error!("Failed to sync db env on shutdown: {:?}", err);
                }
            }

            tracing::info!("Shutting down");

            self.parent_shutdown();
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

    pub fn entity_tracker(&self) -> &EntityTracker {
        self.imp().entity_tracker.get().unwrap()
    }

    pub fn env(&self) -> &heed::Env {
        self.imp().env.get().unwrap()
    }

    fn window(&self) -> Window {
        self.active_window()
            .map_or_else(|| Window::new(self), |w| w.downcast().unwrap())
    }
}

fn init_env() -> Result<(heed::Env, EntityTracker)> {
    let env = db::new_env()?;

    let entity_tracker = EntityTracker::load_from_env(env.clone())?;

    Ok((env, entity_tracker))
}
