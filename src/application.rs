use adw::{prelude::*, subclass::prelude::*};
use anyhow::Result;
use gtk::{
    gio,
    glib::{self, clone},
};

use crate::{
    db,
    detector::Detector,
    entity_id::EntityId,
    settings::Settings,
    timeline::Timeline,
    ui::{EntryWindow, TestWindow, Window, WormholeWindow},
    APP_ID, GRESOURCE_PREFIX,
};

mod imp {
    use std::cell::OnceCell;

    use super::*;

    #[derive(Default)]
    pub struct Application {
        pub(super) settings: Settings,
        pub(super) detector: Detector,
        pub(super) timeline: OnceCell<Timeline>,
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
                Ok((env, timeline)) => {
                    self.env.set(env).unwrap();
                    self.timeline.set(timeline).unwrap();
                }
                Err(err) => {
                    tracing::debug!("Failed to init env: {:?}", err);
                    obj.quit();
                }
            }

            WormholeWindow::init_premade_connection();

            obj.setup_actions();
            obj.setup_accels();

            self.detector.connect_detected(clone!(
                #[weak]
                obj,
                move |_, entity_id| {
                    glib::spawn_future_local(clone!(
                        #[strong]
                        entity_id,
                        async move {
                            obj.handle_detected(&entity_id).await;
                        }
                    ));
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
            .property("resource-base-path", GRESOURCE_PREFIX)
            .build()
    }

    pub fn get() -> Self {
        debug_assert!(
            gtk::is_initialized_main_thread(),
            "application must only be accessed in the main thread"
        );

        gio::Application::default().unwrap().downcast().unwrap()
    }

    pub fn settings(&self) -> &Settings {
        &self.imp().settings
    }

    pub fn detector(&self) -> &Detector {
        &self.imp().detector
    }

    pub fn timeline(&self) -> &Timeline {
        self.imp().timeline.get().unwrap()
    }

    pub fn env(&self) -> &heed::Env {
        self.imp().env.get().unwrap()
    }

    pub fn present_test_window(&self) {
        TestWindow::new(self).present();
    }

    pub fn add_message_toast(&self, message: &str) {
        self.window().add_toast(adw::Toast::new(message));
    }

    fn window(&self) -> Window {
        self.windows()
            .into_iter()
            .find_map(|w| w.downcast::<Window>().ok())
            .unwrap_or_else(|| Window::new(self))
    }

    async fn handle_detected(&self, entity_id: &EntityId) {
        let data = EntryWindow::gather_data(&self.window()).await;

        // TODO If the mode is inventory or refrigerator, don't handle the detected entity
        // if it doesn't have a stock id.
        if let Err(err) = self
            .timeline()
            .handle_detected(entity_id, data.stock_id.as_ref())
        {
            tracing::error!("Failed to handle entity: {:?}", err);
        }
    }

    fn setup_actions(&self) {
        let show_test_window_action = gio::ActionEntry::builder("show-test-window")
            .activate(|obj: &Self, _, _| {
                obj.present_test_window();
            })
            .build();
        let quit_action = gio::ActionEntry::builder("quit")
            .activate(|obj: &Self, _, _| {
                obj.quit();
            })
            .build();
        self.add_action_entries([show_test_window_action, quit_action]);
    }

    fn setup_accels(&self) {
        self.set_accels_for_action("app.quit", &["<Control>q"]);
        self.set_accels_for_action("app.show-test-window", &["<Control><Shift>r"]);
        self.set_accels_for_action("window.close", &["<Control>w"]);
    }
}

fn init_env() -> Result<(heed::Env, Timeline)> {
    let env = db::new_env()?;

    let timeline = Timeline::load_from_env(env.clone())?;

    Ok((env, timeline))
}
