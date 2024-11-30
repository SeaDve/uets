use std::time::Duration;

use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};
use heck::ToTitleCase;
use serde::{Deserialize, Serialize};

use crate::{
    camera::{Camera, CameraState},
    config,
    entity_data::{EntityData, EntityDataField},
    entity_id::EntityId,
    Application,
};

const CAMERA_LAST_DETECTED_RESET_DELAY: Duration = Duration::from_secs(2);

mod imp {
    use std::{cell::RefCell, sync::OnceLock};

    use gtk::glib::subclass::Signal;

    use super::*;

    #[derive(Default)]
    pub struct Detector {
        pub(super) camera: Camera,

        pub(super) camera_last_detected: RefCell<Option<EntityId>>,
        pub(super) camera_last_detected_timeout: RefCell<Option<glib::SourceId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Detector {
        const NAME: &'static str = "UetsDetector";
        type Type = super::Detector;
    }

    impl ObjectImpl for Detector {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            if !config::disable_camera_detection() {
                glib::spawn_future_local(clone!(
                    #[weak]
                    obj,
                    async move {
                        let imp = obj.imp();

                        loop {
                            if matches!(
                                imp.camera.state(),
                                CameraState::Idle | CameraState::Error { .. }
                            ) {
                                if let Err(err) = imp.camera.start().await {
                                    tracing::error!("Failed to start camera: {:?}", err);
                                }
                            }

                            glib::timeout_future(Duration::from_secs(3)).await;
                        }
                    }
                ));

                self.camera.connect_code_detected(clone!(
                    #[weak]
                    obj,
                    move |_, qrcode| {
                        let imp = obj.imp();

                        let Some((id, data)) = entity_from_qrcode(qrcode) else {
                            tracing::warn!("Invalid entity code: {}", qrcode);
                            return;
                        };

                        if imp
                            .camera_last_detected
                            .borrow()
                            .as_ref()
                            .is_some_and(|last_detected| last_detected == &id)
                        {
                            return;
                        }

                        obj.emit_detected(&id, Some(&data));

                        imp.camera_last_detected.replace(Some(id));
                        obj.restart_camera_last_detected_timeout();
                    }
                ));
            }
        }

        fn dispose(&self) {
            self.camera.stop();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| {
                vec![Signal::builder("detected")
                    .param_types([EntityId::static_type(), Option::<EntityData>::static_type()])
                    .build()]
            })
        }
    }
}

glib::wrapper! {
    pub struct Detector(ObjectSubclass<imp::Detector>);
}

impl Detector {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn connect_detected<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &EntityId, Option<EntityData>) + 'static,
    {
        self.connect_closure(
            "detected",
            false,
            closure_local!(|obj: &Self, id: &EntityId, data: Option<EntityData>| f(obj, id, data)),
        )
    }

    pub fn camera(&self) -> &Camera {
        &self.imp().camera
    }

    pub fn simulate_detected(&self, id: &EntityId, data: Option<&EntityData>) {
        self.emit_detected(id, data);
    }

    fn emit_detected(&self, id: &EntityId, data: Option<&EntityData>) {
        self.emit_by_name("detected", &[id, &data])
    }

    fn restart_camera_last_detected_timeout(&self) {
        let imp = self.imp();

        if let Some(source_id) = imp.camera_last_detected_timeout.take() {
            source_id.remove();
        }

        let source_id = glib::timeout_add_local_once(
            CAMERA_LAST_DETECTED_RESET_DELAY,
            clone!(
                #[weak(rename_to = obj)]
                self,
                move || {
                    let imp = obj.imp();
                    imp.camera_last_detected_timeout.replace(None);

                    imp.camera_last_detected.replace(None);
                },
            ),
        );
        imp.camera_last_detected_timeout.replace(Some(source_id));
    }
}

impl Default for Detector {
    fn default() -> Self {
        Self::new()
    }
}

fn entity_from_qrcode(code: &str) -> Option<(EntityId, EntityData)> {
    entity_from_national_id(code).or_else(|| entity_from_qrifying_cea(code))
}

fn entity_from_qrifying_cea(code: &str) -> Option<(EntityId, EntityData)> {
    if !Application::get()
        .settings()
        .operation_mode()
        .is_for_person()
    {
        tracing::debug!("Operation mode is not for person");

        return None;
    }

    let mut substrings = code.splitn(4, '_');
    let name = substrings.next()?;
    let student_id = substrings.next()?;
    let bpsu_email = substrings.next()?;
    let program = substrings.next()?;

    Some((
        EntityId::new(student_id),
        EntityData::from_fields([
            EntityDataField::Name(name.to_string()),
            EntityDataField::Email(bpsu_email.to_string()),
            EntityDataField::Program(program.to_string()),
        ]),
    ))
}

fn entity_from_national_id(code: &str) -> Option<(EntityId, EntityData)> {
    if !Application::get()
        .settings()
        .operation_mode()
        .is_for_person()
    {
        tracing::debug!("Operation mode is not for person");

        return None;
    }

    #[derive(Serialize, Deserialize)]
    pub struct Subject {
        #[serde(rename = "lName")]
        last_name: String,
        #[serde(rename = "fName")]
        first_name: String,
        #[serde(rename = "mName")]
        middle_name: String,
        #[serde(rename = "sex")]
        sex: String,
        #[serde(rename = "DOB")]
        date_of_birth: String,
        #[serde(rename = "POB")]
        place_of_birth: String,
        #[serde(rename = "PCN")]
        pcn: String,
    }

    #[derive(Serialize, Deserialize)]
    pub struct Data {
        #[serde(rename = "DateIssued")]
        date_issued: String,
        #[serde(rename = "Issuer")]
        issuer: String,
        #[serde(rename = "subject")]
        subject: Subject,
    }

    let data = serde_json::from_str::<Data>(code)
        .inspect_err(|err| tracing::debug!("Failed to deserialize national id data: {:?}", err))
        .ok()?;

    Some((
        EntityId::new(data.subject.pcn),
        EntityData::from_fields([
            EntityDataField::Name(format!(
                "{}, {} {}",
                data.subject.last_name.to_title_case(),
                data.subject.first_name.to_title_case(),
                data.subject
                    .middle_name
                    .chars()
                    .next()
                    .map(|c| format!("{}.", c.to_uppercase()))
                    .unwrap_or_default(),
            )),
            EntityDataField::Sex(data.subject.sex),
        ]),
    ))
}
