use adw::{prelude::*, subclass::prelude::*};
use anyhow::Result;
use futures_channel::oneshot;
use gtk::{
    gio,
    glib::{self, clone},
};

use crate::{
    db,
    detector::Detector,
    entity_data::EntityData,
    entity_id::EntityId,
    settings::{OperationMode, Settings},
    timeline::Timeline,
    ui::{EntryWindow, SendWindow, TestWindow, Window},
    APP_ID, GRESOURCE_PREFIX,
};

mod imp {
    use std::cell::OnceCell;

    use super::*;

    #[derive(Default)]
    pub struct Application {
        pub(super) settings: Settings,
        pub(super) detector: Detector,
        pub(super) env: OnceCell<heed::Env>,
        pub(super) timeline: OnceCell<Timeline>,
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

            SendWindow::init_premade_connection();

            obj.setup_actions();
            obj.setup_accels();

            self.detector.connect_detected(clone!(
                #[weak]
                obj,
                move |_, entity_id, entity_data| {
                    glib::spawn_future_local(clone!(
                        #[strong]
                        entity_id,
                        #[strong]
                        entity_data,
                        async move {
                            obj.handle_detected(&entity_id, entity_data).await;
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

    pub fn env(&self) -> &heed::Env {
        self.imp().env.get().unwrap()
    }

    pub fn timeline(&self) -> &Timeline {
        self.imp().timeline.get().unwrap()
    }

    pub fn present_test_window(&self) {
        TestWindow::new(self).present();
    }

    pub fn add_message_toast(&self, message: &str) {
        self.window().add_toast(adw::Toast::new(message));
    }

    pub fn window(&self) -> Window {
        self.windows()
            .into_iter()
            .find_map(|w| w.downcast::<Window>().ok())
            .unwrap_or_else(|| Window::new(self))
    }

    async fn handle_detected(&self, entity_id: &EntityId, entity_data: Option<EntityData>) {
        let timeline = self.timeline();

        let data = if let Some(data) = entity_data {
            tracing::debug!("Using entity data from detector");

            data
        } else if let Some(entity) = timeline.entity_list().get(entity_id) {
            tracing::debug!("Retrieved entity data from timeline");

            entity.data().clone()
        } else if self.settings().operation_mode() != OperationMode::Counter {
            tracing::debug!("Gathering entity data from user");

            match EntryWindow::gather_data(Some(&self.window())).await {
                Ok(data) => data,
                Err(oneshot::Canceled) => {
                    tracing::debug!("Gathering entity data was canceled; ignoring detected entity");
                    return;
                }
            }
        } else {
            tracing::debug!("Using empty entity data for counter mode");

            EntityData::new()
        };

        tracing::debug!(?data, "Handling detected entity `{}`", entity_id);

        // TODO If the mode is inventory or refrigerator, don't handle the detected entity
        // if it doesn't have a stock id.
        if let Err(err) = timeline.handle_detected(entity_id, data) {
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
