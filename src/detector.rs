use std::time::Duration;

use chrono::Utc;
use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};
use heck::ToTitleCase;
use serde::{Deserialize, Serialize};

use crate::{
    camera::Camera,
    date_time_boxed::DateTimeBoxed,
    entity_data::{EntityData, EntityDataField},
    entity_id::EntityId,
    jpeg_image::JpegImage,
    remote::Remote,
    rfid_reader::RfidReader,
    sex::Sex,
    Application,
};

const CAMERA_LAST_DETECTED_RESET_DELAY: Duration = Duration::from_secs(2);
const DETECTED_WO_ID_ALERT_DELAY: Duration = Duration::from_secs(5);

mod imp {
    use std::{cell::RefCell, sync::OnceLock};

    use gtk::glib::subclass::Signal;

    use super::*;

    #[derive(Default)]
    pub struct Detector {
        pub(super) camera: RefCell<Option<Camera>>,
        pub(super) aux_cameras: RefCell<Vec<(Camera, Vec<glib::SignalHandlerId>)>>,
        pub(super) camera_last_detected: RefCell<Option<String>>,
        pub(super) camera_last_detected_timeout: RefCell<Option<glib::SourceId>>,

        pub(super) detected_wo_id_capture: RefCell<Option<(DateTimeBoxed, Option<JpegImage>)>>,
        pub(super) detected_wo_id_alert_timeout: RefCell<Option<glib::SourceId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Detector {
        const NAME: &'static str = "UetsDetector";
        type Type = super::Detector;
    }

    impl ObjectImpl for Detector {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("detected")
                        .param_types([EntityId::static_type(), Option::<EntityData>::static_type()])
                        .build(),
                    Signal::builder("detected-invalid")
                        .param_types([String::static_type()])
                        .build(),
                    Signal::builder("detected-wo-id")
                        .param_types([
                            DateTimeBoxed::static_type(),
                            Option::<JpegImage>::static_type(),
                        ])
                        .build(),
                ]
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

    pub fn connect_detected_invalid<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &str) + 'static,
    {
        self.connect_closure(
            "detected-invalid",
            false,
            closure_local!(|obj: &Self, code: &str| f(obj, code)),
        )
    }

    pub fn connect_detected_wo_id<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &DateTimeBoxed, Option<&JpegImage>) + 'static,
    {
        self.connect_closure(
            "detected-wo-id",
            false,
            closure_local!(
                |obj: &Self, dt: &DateTimeBoxed, image: Option<&JpegImage>| f(obj, dt, image)
            ),
        )
    }

    pub fn bind_camera(&self, camera: &Camera) {
        let imp = self.imp();

        self.bind_camera_inner(camera);

        imp.camera.replace(Some(camera.clone()));
    }

    pub fn bind_aux_cameras(&self, cameras: &[Camera]) {
        let imp = self.imp();

        for camera in cameras {
            let handler_ids = self.bind_camera_inner(camera);
            imp.aux_cameras
                .borrow_mut()
                .push((camera.clone(), handler_ids));
        }

        tracing::debug!(
            cameras = ?cameras.iter().map(|c| c.ip_addr()).collect::<Vec<_>>(),
            "Bound aux cameras"
        );
    }

    pub fn unbind_aux_cameras(&self) {
        let imp = self.imp();

        for (camera, handler_ids) in imp.aux_cameras.take() {
            for handler_id in handler_ids {
                camera.disconnect(handler_id);
            }
        }
    }

    pub fn aux_cameras(&self) -> Vec<Camera> {
        self.imp()
            .aux_cameras
            .borrow()
            .iter()
            .map(|(camera, _)| camera.clone())
            .collect()
    }

    pub fn bind_rfid_reader(&self, rfid_reader: &RfidReader) {
        rfid_reader.connect_detected(clone!(
            #[weak(rename_to = obj)]
            self,
            move |_, id| {
                let entity_id = EntityId::new(id);
                obj.emit_detected(&entity_id, None);
            }
        ));
    }

    pub fn simulate_detected(&self, id: &EntityId, data: Option<&EntityData>) {
        self.emit_detected(id, data);
    }

    pub fn set_enable_detection_wo_id(&self, is_enabled: bool) {
        let imp = self.imp();

        if let Some(camera) = imp.camera.borrow().as_ref() {
            camera.set_enable_motion_detection(is_enabled);
        }

        for (camera, _) in imp.aux_cameras.borrow().iter() {
            camera.set_enable_motion_detection(is_enabled);
        }
    }

    fn emit_detected(&self, id: &EntityId, data: Option<&EntityData>) {
        self.emit_by_name::<()>("detected", &[id, &data]);

        self.stop_detected_wo_id_alert_timeout();
    }

    fn bind_camera_inner(&self, camera: &Camera) -> Vec<glib::SignalHandlerId> {
        let handler_ids = vec![
            camera.connect_code_detected(clone!(
                #[weak(rename_to = obj)]
                self,
                move |_, code| {
                    let imp = obj.imp();

                    if imp
                        .camera_last_detected
                        .borrow()
                        .as_ref()
                        .is_some_and(|last_detected| last_detected == code)
                    {
                        return;
                    }

                    tracing::debug!("Detected code: {}", code);

                    if let Some((id, data)) = entity_from_qrcode(code) {
                        obj.emit_detected(&id, Some(&data));
                    } else {
                        obj.emit_by_name::<()>("detected-invalid", &[&code]);
                    }

                    imp.camera_last_detected.replace(Some(code.to_string()));
                    obj.restart_camera_last_detected_timeout();
                }
            )),
            camera.connect_motion_detected(clone!(
                #[weak(rename_to = obj)]
                self,
                move |camera| {
                    glib::spawn_future_local(clone!(
                        #[strong]
                        obj,
                        #[strong]
                        camera,
                        async move {
                            let imp = obj.imp();

                            let now = Utc::now();

                            let image = camera
                                .capture_jpeg()
                                .await
                                .inspect_err(|err| {
                                    tracing::warn!("Failed to capture image: {:?}", err)
                                })
                                .ok();

                            imp.detected_wo_id_capture
                                .replace(Some((DateTimeBoxed(now), image)));

                            if imp.detected_wo_id_alert_timeout.borrow().is_some() {
                                obj.detected_wo_id_alert();
                            } else {
                                obj.start_detected_wo_id_alert_timeout();
                            }
                        }
                    ));
                }
            )),
        ];

        if let Err(err) = camera.start() {
            tracing::error!("Failed to start camera: {:?}", err);
        }

        handler_ids
    }

    fn start_detected_wo_id_alert_timeout(&self) {
        let imp = self.imp();

        let source_id = glib::timeout_add_local_once(
            DETECTED_WO_ID_ALERT_DELAY,
            clone!(
                #[weak(rename_to = obj)]
                self,
                move || {
                    let imp = obj.imp();
                    imp.detected_wo_id_alert_timeout.replace(None);

                    obj.detected_wo_id_alert();
                },
            ),
        );
        imp.detected_wo_id_alert_timeout.replace(Some(source_id));
    }

    fn stop_detected_wo_id_alert_timeout(&self) {
        let imp = self.imp();

        if let Some(source_id) = imp.detected_wo_id_alert_timeout.take() {
            source_id.remove();
        }

        imp.detected_wo_id_capture.replace(None);
    }

    fn detected_wo_id_alert(&self) {
        let imp = self.imp();

        if let Some((dt, image)) = imp.detected_wo_id_capture.take() {
            self.emit_by_name::<()>("detected-wo-id", &[&dt, &image]);
        } else {
            tracing::warn!("No detected without ID capture data");
        }
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
        tracing::trace!("Operation mode is not for person");

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
        tracing::trace!("Operation mode is not for person");

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

    let mut fields = vec![EntityDataField::Name(format!(
        "{}, {} {}",
        data.subject.last_name.to_title_case(),
        data.subject.first_name.to_title_case(),
        data.subject
            .middle_name
            .chars()
            .next()
            .map(|c| format!("{}.", c.to_uppercase()))
            .unwrap_or_default(),
    ))];

    match data.subject.sex.parse::<Sex>() {
        Ok(sex) => fields.push(EntityDataField::Sex(sex)),
        Err(err) => tracing::warn!("Failed to parse sex: {:?}", err),
    }

    Some((
        EntityId::new(data.subject.pcn),
        EntityData::from_fields(fields),
    ))
}
