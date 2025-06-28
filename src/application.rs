use std::iter;

use adw::{prelude::*, subclass::prelude::*};
use anyhow::Result;
use futures_channel::oneshot;
use gtk::{
    gio,
    glib::{self, clone},
};

use crate::{
    camera::Camera,
    date_time_boxed::DateTimeBoxed,
    date_time_updater::DateTimeUpdater,
    db,
    detected_wo_id_item::DetectedWoIdItem,
    detected_wo_id_list::DetectedWoIdList,
    detector::Detector,
    entity::Entity,
    entity_data::{EntityData, EntityDataField, EntityDataFieldTy},
    entity_entry_tracker::EntityIdSet,
    entity_id::EntityId,
    entity_kind::EntityKind,
    jpeg_image::JpegImage,
    limit_reached::{LimitReached, LimitReachedSettingsExt},
    relay::{Relay, RelayState},
    settings::Settings,
    sound::Sound,
    timeline::Timeline,
    timeline_item_kind::TimelineItemKind,
    ui::{EntityDataDialog, SendDialog, TestWindow, ToastId, Window},
    APP_ID, GRESOURCE_PREFIX,
};

mod imp {
    use std::cell::{OnceCell, RefCell};

    use super::*;

    #[derive(Default)]
    pub struct Application {
        pub(super) settings: Settings,

        pub(super) date_time_updater: DateTimeUpdater,

        pub(super) camera: OnceCell<Camera>,
        pub(super) detectors: RefCell<Vec<(Detector, Vec<glib::SignalHandlerId>)>>,
        pub(super) relay: OnceCell<Relay>,

        pub(super) env: OnceCell<heed::Env>,
        pub(super) timeline: OnceCell<Timeline>,
        pub(super) detected_wo_id_list: OnceCell<DetectedWoIdList>,
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

            SendDialog::init_premade_connection();

            self.settings
                .connect_limit_reached_threshold_changed(clone!(
                    #[weak]
                    obj,
                    move |_| {
                        obj.alert_if_limit_reached();
                    }
                ));
            self.settings
                .connect_enable_lower_limit_reached_alert_changed(clone!(
                    #[weak]
                    obj,
                    move |_| {
                        obj.alert_if_limit_reached();
                    }
                ));
            self.settings
                .connect_enable_upper_limit_reached_alert_changed(clone!(
                    #[weak]
                    obj,
                    move |_| {
                        obj.alert_if_limit_reached();
                    }
                ));

            self.settings.connect_detector_config_changed(clone!(
                #[weak]
                obj,
                move |_| {
                    if let Err(err) = obj.reconfigure_detectors() {
                        tracing::error!("Failed to reconfigure detectors: {:?}", err);
                    }
                }
            ));

            self.settings.connect_camera_ip_addr_changed(clone!(
                #[weak]
                obj,
                move |settings| {
                    let ip_addr = settings.camera_ip_addr();
                    if let Err(err) = obj.camera().set_ip_addr(ip_addr) {
                        tracing::error!("Failed to set camera IP address: {:?}", err);
                    }
                }
            ));
            self.settings.connect_relay_ip_addr_changed(clone!(
                #[weak]
                obj,
                move |settings| {
                    let ip_addr = settings.relay_ip_addr();
                    obj.relay().set_ip_addr(ip_addr);
                }
            ));
            self.settings.connect_enable_detection_wo_id_changed(clone!(
                #[weak]
                obj,
                move |settings| {
                    for detector in obj.detectors() {
                        detector.set_enable_detection_wo_id(settings.enable_detection_wo_id());
                    }
                }
            ));
            self.settings.connect_enable_n_inside_hook_changed(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_relay_state();
                }
            ));
            self.settings
                .connect_n_inside_hook_threshold_changed(clone!(
                    #[weak]
                    obj,
                    move |_| {
                        obj.update_relay_state();
                    }
                ));

            let camera = Camera::new(self.settings.camera_ip_addr());
            self.camera.set(camera).unwrap();

            let relay = Relay::new(self.settings.relay_ip_addr());
            self.relay.set(relay).unwrap();

            match init_env() {
                Ok((env, timeline, detected_wo_id_list)) => {
                    self.env.set(env).unwrap();
                    self.timeline.set(timeline).unwrap();
                    self.detected_wo_id_list.set(detected_wo_id_list).unwrap();
                }
                Err(err) => {
                    tracing::debug!("Failed to init env: {:?}", err);
                    obj.quit();
                }
            }

            obj.timeline().connect_n_inside_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_relay_state();
                    obj.alert_if_limit_reached();
                }
            ));
            obj.timeline()
                .entity_entry_tracker()
                .connect_overstayed(clone!(
                    #[weak]
                    obj,
                    move |_, EntityIdSet(entity_ids)| {
                        if entity_ids.is_empty() {
                            return;
                        }

                        match entity_ids.iter().collect::<Vec<_>>().as_slice() {
                            [] => return,
                            [id] => {
                                let entity = obj
                                    .timeline()
                                    .entity_list()
                                    .get(id)
                                    .expect("entity must exist");

                                obj.add_message_toast(&format!(
                                    "“{}” overstayed",
                                    id_or_name(&entity)
                                ));
                            }
                            [id1, id2] => {
                                let entity1 = obj
                                    .timeline()
                                    .entity_list()
                                    .get(id1)
                                    .expect("entity must exist");
                                let entity2 = obj
                                    .timeline()
                                    .entity_list()
                                    .get(id2)
                                    .expect("entity must exist");

                                obj.add_message_toast(&format!(
                                    "“{}” and “{}” overstayed",
                                    id_or_name(&entity1),
                                    id_or_name(&entity2),
                                ));
                            }
                            ids => {
                                obj.add_message_toast(&format!(
                                    "{} entities overstayed",
                                    ids.len()
                                ));
                            }
                        }

                        Sound::CriticalAlert.play();
                    }
                ));

            obj.setup_actions();
            obj.setup_accels();

            obj.alert_if_limit_reached();

            if let Err(err) = obj.reconfigure_detectors() {
                tracing::error!("Failed to reconfigure detectors: {:?}", err);
            }

            obj.update_relay_state();
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

    pub fn date_time_updater(&self) -> &DateTimeUpdater {
        &self.imp().date_time_updater
    }

    pub fn camera(&self) -> &Camera {
        self.imp().camera.get().unwrap()
    }

    pub fn detectors(&self) -> Vec<Detector> {
        self.imp()
            .detectors
            .borrow()
            .iter()
            .map(|(d, _)| d.clone())
            .collect()
    }

    pub fn relay(&self) -> &Relay {
        self.imp().relay.get().unwrap()
    }

    pub fn env(&self) -> &heed::Env {
        self.imp().env.get().unwrap()
    }

    pub fn timeline(&self) -> &Timeline {
        self.imp().timeline.get().unwrap()
    }

    pub fn detected_wo_id_list(&self) -> &DetectedWoIdList {
        self.imp().detected_wo_id_list.get().unwrap()
    }

    pub fn window(&self) -> Window {
        self.windows()
            .into_iter()
            .find_map(|w| w.downcast::<Window>().ok())
            .unwrap_or_else(|| Window::new(self))
    }

    pub fn present_test_window(&self) {
        TestWindow::new(self).present();
    }

    pub fn add_message_toast(&self, message: &str) {
        self.window().add_message_toast(message);
    }

    pub fn add_message_toast_with_id(&self, id: ToastId, message: &str) {
        self.window().add_message_toast_with_id(id, message);
    }

    pub fn remove_message_toast_with_id(&self, id: ToastId) {
        self.window().remove_message_toast_with_id(id);
    }

    pub async fn simulate_detected(
        &self,
        entity_id: &EntityId,
        entity_data_fields: Vec<EntityDataField>,
    ) -> Option<String> {
        self.handle_detected(entity_id, entity_data_fields).await
    }

    pub fn reconfigure_detectors(&self) -> Result<()> {
        let imp = self.imp();

        let settings = self.settings();

        for (detector, handler_ids) in imp.detectors.take() {
            for handler_id in handler_ids {
                detector.disconnect(handler_id);
            }
        }

        for config in settings.detector_config_parsed()? {
            let detector = Detector::new(config, self.timeline());
            detector.set_enable_detection_wo_id(settings.enable_detection_wo_id());

            let handler_ids = vec![
                detector.connect_detected(clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |detector, entity_id, entity_data_fields| {
                        let entity_data_fields = entity_data_fields.0.clone();
                        glib::spawn_future_local(clone!(
                            #[weak]
                            obj,
                            #[weak]
                            detector,
                            #[strong]
                            entity_id,
                            async move {
                                if let Some(message) =
                                    obj.handle_detected(&entity_id, entity_data_fields).await
                                {
                                    if let Err(err) = detector.return_message(&message).await {
                                        tracing::error!("Failed to return message: {:?}", err);
                                    }
                                }
                            }
                        ));
                    }
                )),
                detector.connect_detected_invalid(clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |detector, code| {
                        obj.handle_detected_invalid(detector, code);
                    }
                )),
                detector.connect_detected_wo_id(clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |detector, dt, image| {
                        if let Err(err) = obj.handle_detected_wo_id(detector, dt, image) {
                            tracing::error!("Failed to handle detected wo id: {:?}", err);
                        }
                    }
                )),
            ];

            imp.detectors.borrow_mut().push((detector, handler_ids));
        }

        Ok(())
    }

    fn alert_if_limit_reached(&self) {
        let settings = self.settings();

        match settings.compute_limit_reached(self.timeline().n_inside()) {
            Some(LimitReached::Lower) if settings.enable_lower_limit_reached_alert() => {
                self.add_message_toast_with_id(ToastId::LimitReached, "Amount Depleted");

                Sound::CriticalAlert.play();
            }
            Some(LimitReached::Upper) if settings.enable_upper_limit_reached_alert() => {
                self.add_message_toast_with_id(ToastId::LimitReached, "Capacity Exceeded");

                Sound::CriticalAlert.play();
            }
            _ => {
                self.remove_message_toast_with_id(ToastId::LimitReached);
            }
        }
    }

    async fn handle_detected(
        &self,
        detected_entity_id: &EntityId,
        detected_entity_data_fields: Vec<EntityDataField>,
    ) -> Option<String> {
        let timeline = self.timeline();

        let data = match timeline.entity_list().get(detected_entity_id) {
            Some(entity) => {
                tracing::debug!("Retrieved entity data from timeline");

                entity.data().clone().extended(detected_entity_data_fields)
            }
            None if !detected_entity_data_fields.is_empty() => {
                tracing::debug!("Using entity data from detector");

                if detected_entity_data_fields
                    .iter()
                    .any(|f| f.ty() == EntityDataFieldTy::Kind)
                {
                    EntityData::from_fields(detected_entity_data_fields)
                } else {
                    tracing::debug!("Detected entity data has no `kind` field; using default kind");

                    EntityData::from_fields(
                        detected_entity_data_fields
                            .into_iter()
                            .chain(iter::once(EntityDataField::Kind(EntityKind::default()))),
                    )
                }
            }
            None => {
                tracing::debug!("Gathering entity data from user");

                match EntityDataDialog::gather_data(
                    detected_entity_id,
                    &EntityData::from_fields([EntityDataField::Kind(EntityKind::default())]),
                    [],
                    Some(&self.window()),
                )
                .await
                {
                    Ok(data) => data.extended(detected_entity_data_fields),
                    Err(oneshot::Canceled) => {
                        tracing::debug!(
                            "Gathering entity data was canceled; ignoring detected entity"
                        );
                        return None;
                    }
                }
            }
        };

        tracing::debug!(?data, "Handling detected entity `{}`", detected_entity_id);

        // TODO If the mode is inventory, don't handle the detected entity
        // if it doesn't have a stock id.
        let entity_name = data.name().cloned();
        let entity_kind = data.kind();
        let entity_possessor_title = data.possessor().map(|possessor| {
            let possessor_entity = self
                .timeline()
                .entity_list()
                .get(possessor)
                .expect("possessor should exist");
            possessor_entity
                .data()
                .name()
                .cloned()
                .unwrap_or_else(|| possessor.to_string())
        });
        match timeline.handle_detected(detected_entity_id, data) {
            Ok(item) => {
                let welcome_message = match item.kind() {
                    TimelineItemKind::Entry => match entity_name {
                        Some(name) if entity_kind == EntityKind::Person => {
                            format!("Welcome, {}!", name)
                        }
                        Some(name) => {
                            format!(
                                "{name} {}",
                                entity_possessor_title.map_or_else(
                                    || entity_kind.enter_verb().to_string(),
                                    |p| entity_kind.enter_verb_with_possessor(&p),
                                )
                            )
                        }
                        None => {
                            format!(
                                "{detected_entity_id} {}",
                                entity_possessor_title.map_or_else(
                                    || entity_kind.enter_verb().to_string(),
                                    |p| entity_kind.enter_verb_with_possessor(&p),
                                )
                            )
                        }
                    },
                    TimelineItemKind::Exit => match entity_name {
                        Some(name) if entity_kind == EntityKind::Person => {
                            format!("Goodbye, {}!", name)
                        }
                        Some(name) => {
                            format!(
                                "{name} {}",
                                entity_possessor_title.map_or_else(
                                    || entity_kind.exit_verb().to_string(),
                                    |p| entity_kind.exit_verb_with_possessor(&p),
                                )
                            )
                        }
                        None => {
                            format!(
                                "{detected_entity_id} {}",
                                entity_possessor_title.map_or_else(
                                    || entity_kind.exit_verb().to_string(),
                                    |p| entity_kind.exit_verb_with_possessor(&p),
                                )
                            )
                        }
                    },
                };

                let entity = timeline
                    .entity_list()
                    .get(item.entity_id())
                    .expect("entity must exist");

                if !entity
                    .data()
                    .allowed_dt_range()
                    .copied()
                    .unwrap_or_default()
                    .contains(item.dt())
                    && item.kind().is_entry()
                {
                    let message = format!("“{}” is not allowed!", id_or_name(&entity));
                    self.add_message_toast_with_id(ToastId::Detected, &message);

                    Sound::CriticalAlert.play();

                    Some(message)
                } else {
                    self.add_message_toast_with_id(ToastId::Detected, &welcome_message);

                    Sound::DetectedSuccess.play();

                    Some(welcome_message)
                }
            }
            Err(err) => {
                tracing::error!("Failed to handle entity: {:?}", err);

                let message = format!("Failed to handle “{}”", detected_entity_id);
                self.add_message_toast_with_id(ToastId::Detected, &message);

                Sound::DetectedError.play();

                Some(message)
            }
        }
    }

    fn handle_detected_invalid(&self, detector: &Detector, _code: &str) {
        Sound::DetectedError.play();

        self.add_message_toast(&format!("Invalid code detected on {}", detector.name()));

        glib::spawn_future_local(clone!(
            #[weak]
            detector,
            async move {
                if let Err(err) = detector.return_message("Invalid code detected").await {
                    tracing::error!("Failed to return message: {:?}", err);
                }
            }
        ));
    }

    fn handle_detected_wo_id(
        &self,
        detector: &Detector,
        dt: &DateTimeBoxed,
        image: Option<&JpegImage>,
    ) -> Result<()> {
        Sound::CriticalAlert.play();

        self.add_message_toast("Detected unregistered entity!");

        let item = DetectedWoIdItem::new(dt.0, detector.name().to_string(), image.cloned());
        self.detected_wo_id_list().insert(item)?;

        Ok(())
    }

    fn update_relay_state(&self) {
        glib::spawn_future_local(clone!(
            #[strong(rename_to = obj)]
            self,
            async move {
                if let Err(err) = obj.update_relay_state_inner().await {
                    tracing::error!("Failed to update relay state: {:?}", err);
                }
            }
        ));
    }

    async fn update_relay_state_inner(&self) -> Result<()> {
        let settings = self.settings();

        let state = if settings.enable_n_inside_hook()
            && self.timeline().n_inside() > settings.n_inside_hook_threshold()
        {
            RelayState::High
        } else {
            RelayState::Low
        };

        self.relay().set_state(state).await?;

        Ok(())
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

fn init_env() -> Result<(heed::Env, Timeline, DetectedWoIdList)> {
    let env = db::new_env()?;

    let timeline = Timeline::load_from_env(env.clone())?;
    let detected_wo_id_list = DetectedWoIdList::load_from_env(env.clone())?;

    Ok((env, timeline, detected_wo_id_list))
}

fn id_or_name(entity: &Entity) -> String {
    entity
        .data()
        .name()
        .map_or_else(|| entity.id().to_string(), |n| n.clone())
}
