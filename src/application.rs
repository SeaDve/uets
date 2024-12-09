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
    db,
    detected_wo_id_item::DetectedWoIdItem,
    detected_wo_id_list::DetectedWoIdList,
    detector::Detector,
    entity::Entity,
    entity_data::EntityData,
    entity_entry_tracker::EntityIdSet,
    entity_id::EntityId,
    jpeg_image::JpegImage,
    limit_reached::{LimitReached, LimitReachedSettingsExt},
    relay::{Relay, RelayState},
    rfid_reader::RfidReader,
    settings::{OperationMode, Settings},
    sound::Sound,
    timeline::Timeline,
    timeline_item_kind::TimelineItemKind,
    ui::{EntityDataDialog, SendDialog, TestWindow, ToastId, Window},
    APP_ID, GRESOURCE_PREFIX,
};

mod imp {
    use std::cell::OnceCell;

    use super::*;

    #[derive(Default)]
    pub struct Application {
        pub(super) settings: Settings,

        pub(super) camera: OnceCell<Camera>,
        pub(super) rfid_reader: OnceCell<RfidReader>,
        pub(super) detector: Detector,

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
            self.settings.connect_aux_camera_ip_addrs_changed(clone!(
                #[weak]
                obj,
                move |settings| {
                    obj.detector().unbind_aux_cameras();

                    let cameras = settings
                        .aux_camera_ip_addrs()
                        .into_iter()
                        .map(Camera::new)
                        .collect::<Vec<_>>();
                    obj.detector().bind_aux_cameras(&cameras);
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
            self.settings.connect_rfid_reader_ip_addr_changed(clone!(
                #[weak]
                obj,
                move |settings| {
                    let ip_addr = settings.rfid_reader_ip_addr();
                    obj.rfid_reader().set_ip_addr(ip_addr);
                }
            ));
            self.settings.connect_enable_detection_wo_id_changed(clone!(
                #[weak]
                obj,
                move |settings| {
                    obj.detector()
                        .set_enable_detection_wo_id(settings.enable_detection_wo_id());
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

            let aux_cameras = self
                .settings
                .aux_camera_ip_addrs()
                .into_iter()
                .filter(|ip_addr| !ip_addr.is_empty())
                .map(Camera::new)
                .collect::<Vec<_>>();

            let rfid_reader = RfidReader::new(self.settings.rfid_reader_ip_addr());
            self.rfid_reader.set(rfid_reader).unwrap();

            self.detector.bind_camera(obj.camera());
            self.detector.bind_aux_cameras(&aux_cameras);
            self.detector.bind_rfid_reader(obj.rfid_reader());

            self.detector
                .set_enable_detection_wo_id(self.settings.enable_detection_wo_id());
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
            self.detector.connect_detected_invalid(clone!(
                #[weak]
                obj,
                move |_, code| {
                    obj.handle_detected_invalid(code);
                }
            ));
            self.detector.connect_detected_wo_id(clone!(
                #[weak]
                obj,
                move |_, dt, image| {
                    if let Err(err) = obj.handle_detected_wo_id(dt, image) {
                        tracing::error!("Failed to handle detected wo id: {:?}", err);
                    }
                }
            ));

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

            obj.update_relay_state();
        }

        fn shutdown(&self) {
            let obj = self.obj();

            if let Some(env) = self.env.get() {
                if let Err(err) = env.force_sync() {
                    tracing::error!("Failed to sync db env on shutdown: {:?}", err);
                }
            }

            obj.camera().stop();

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

    pub fn camera(&self) -> &Camera {
        self.imp().camera.get().unwrap()
    }

    pub fn rfid_reader(&self) -> &RfidReader {
        self.imp().rfid_reader.get().unwrap()
    }

    pub fn detector(&self) -> &Detector {
        &self.imp().detector
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

    pub fn window(&self) -> Window {
        self.windows()
            .into_iter()
            .find_map(|w| w.downcast::<Window>().ok())
            .unwrap_or_else(|| Window::new(self))
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

            match EntityDataDialog::gather_data(
                entity_id,
                &EntityData::new(),
                [],
                Some(&self.window()),
            )
            .await
            {
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
        let entity_name = data.name().cloned();
        match timeline.handle_detected(entity_id, data) {
            Ok(item) => {
                if let Some(name) = entity_name {
                    match item.kind() {
                        TimelineItemKind::Entry => {
                            self.add_message_toast_with_id(
                                ToastId::Detected,
                                &format!("Welcome, {}!", name),
                            );
                        }
                        TimelineItemKind::Exit => {
                            self.add_message_toast_with_id(
                                ToastId::Detected,
                                &format!("Goodbye, {}!", name),
                            );
                        }
                    }
                }

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
                    self.add_message_toast_with_id(
                        ToastId::Detected,
                        &format!("“{}” is not allowed!", id_or_name(&entity)),
                    );

                    Sound::CriticalAlert.play();
                } else {
                    Sound::DetectedSuccess.play();
                }
            }
            Err(err) => {
                tracing::error!("Failed to handle entity: {:?}", err);

                self.add_message_toast("Can't handle entity");

                Sound::DetectedError.play();
            }
        }
    }

    fn handle_detected_invalid(&self, _code: &str) {
        Sound::DetectedError.play();

        self.add_message_toast("Invalid code detected");
    }

    fn handle_detected_wo_id(&self, dt: &DateTimeBoxed, image: Option<&JpegImage>) -> Result<()> {
        Sound::CriticalAlert.play();

        self.add_message_toast("Detected unregistered entity!");

        let item = DetectedWoIdItem::new(dt.0, image.cloned());
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
