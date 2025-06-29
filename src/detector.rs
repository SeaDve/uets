use std::time::Duration;

use anyhow::Result;
use chrono::Utc;
use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};
use inflections::case;
use serde::{Deserialize, Serialize};

use crate::{
    camera::Camera,
    date_time_boxed::DateTimeBoxed,
    entity_data::{EntityDataField, EntityDataFieldVecBoxed},
    entity_id::EntityId,
    entity_kind::EntityKind,
    jpeg_image::JpegImage,
    remote_app::RemoteApp,
    rfid_reader::RfidReader,
    settings::DetectorConfig,
    sex::Sex,
    timeline::Timeline,
};

const CAMERA_LAST_DETECTED_RESET_DELAY: Duration = Duration::from_secs(2);
const DETECTED_WO_ID_ALERT_DELAY: Duration = Duration::from_secs(5);

mod imp {
    use std::{
        cell::{OnceCell, RefCell},
        sync::OnceLock,
    };

    use gtk::glib::subclass::Signal;

    use super::*;

    #[derive(Default)]
    pub struct Detector {
        pub(super) name: OnceCell<String>,

        pub(super) camera: OnceCell<Camera>,
        pub(super) camera_last_detected: RefCell<Option<String>>,
        pub(super) camera_last_detected_reset_timeout: RefCell<Option<glib::SourceId>>,

        pub(super) rfid_reader: OnceCell<RfidReader>,

        pub(super) remote_app: OnceCell<RemoteApp>,

        pub(super) detected_wo_id_capture: RefCell<Option<(DateTimeBoxed, Option<JpegImage>)>>,
        pub(super) detected_wo_id_alert_timeout: RefCell<Option<glib::SourceId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Detector {
        const NAME: &'static str = "UetsDetector";
        type Type = super::Detector;
    }

    impl ObjectImpl for Detector {
        fn dispose(&self) {
            let obj = self.obj();

            if let Some(camera) = self.camera.get() {
                camera.stop();
            }

            if let Some(rfid_reader) = self.rfid_reader.get() {
                rfid_reader.stop();
            }

            if let Some(remote_app) = self.remote_app.get() {
                remote_app.stop();
            }

            tracing::debug!("Detector `{}` disposed", obj.name());
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("detected")
                        .param_types([
                            EntityId::static_type(),
                            EntityDataFieldVecBoxed::static_type(),
                        ])
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
    pub fn new(config: DetectorConfig, timeline: &Timeline) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.name.set(config.name).unwrap();

        if let Some(ip_addr) = config.camera_ip_addr {
            let camera = Camera::new(ip_addr);

            camera.connect_code_detected(clone!(
                #[weak]
                this,
                move |_, code| {
                    this.handle_code_detected(code);
                }
            ));
            camera.connect_motion_detected(clone!(
                #[weak]
                this,
                move |camera| {
                    glib::spawn_future_local(clone!(
                        #[strong]
                        this,
                        #[strong]
                        camera,
                        async move {
                            let imp = this.imp();

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
                                this.detected_wo_id_alert();
                            } else {
                                this.start_detected_wo_id_alert_timeout();
                            }
                        }
                    ));
                }
            ));

            if let Err(err) = camera.start() {
                tracing::error!("Failed to start camera: {:?}", err);
            }

            imp.camera.set(camera).unwrap();
        }

        if let Some(ip_addr) = config.rfid_reader_ip_addr {
            let rfid_reader = RfidReader::new(ip_addr);
            rfid_reader.connect_detected(clone!(
                #[weak]
                this,
                move |_, id| {
                    let entity_id = EntityId::new(id);
                    this.emit_detected(&entity_id, vec![]);
                }
            ));
            imp.rfid_reader.set(rfid_reader).unwrap();
        }

        if let Some(remote_app) = config.remote_app_ip_addr {
            let remote_app = RemoteApp::new(remote_app);
            remote_app.bind_timeline(timeline);

            remote_app.connect_code_detected(clone!(
                #[weak]
                this,
                move |_, code| {
                    this.handle_code_detected(code);
                }
            ));
            remote_app.connect_entity_detected(clone!(
                #[weak]
                this,
                move |_, id, data_fields| {
                    this.emit_detected(id, data_fields.0.clone());
                }
            ));
            imp.remote_app.set(remote_app).unwrap();
        }

        this
    }

    pub fn connect_detected<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &EntityId, &EntityDataFieldVecBoxed) + 'static,
    {
        self.connect_closure(
            "detected",
            false,
            closure_local!(
                |obj: &Self, id: &EntityId, data_fields_boxed: &EntityDataFieldVecBoxed| f(
                    obj,
                    id,
                    data_fields_boxed
                )
            ),
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

    pub fn name(&self) -> &str {
        self.imp().name.get().unwrap()
    }

    pub fn camera(&self) -> Option<Camera> {
        self.imp().camera.get().cloned()
    }

    pub fn rfid_reader(&self) -> Option<RfidReader> {
        self.imp().rfid_reader.get().cloned()
    }

    pub fn remote_app(&self) -> Option<RemoteApp> {
        self.imp().remote_app.get().cloned()
    }

    pub fn set_enable_detection_wo_id(&self, is_enabled: bool) {
        let imp = self.imp();

        if let Some(camera) = imp.camera.get() {
            camera.set_enable_motion_detection(is_enabled);
        }
    }

    pub async fn return_message(&self, message: &str) -> Result<()> {
        if let Some(remote_app) = self.imp().remote_app.get() {
            remote_app.ws_send_message(message).await?;
        }

        Ok(())
    }

    fn emit_detected(&self, id: &EntityId, data_fields: Vec<EntityDataField>) {
        self.emit_by_name::<()>("detected", &[id, &EntityDataFieldVecBoxed(data_fields)]);

        self.stop_detected_wo_id_alert_timeout();
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

    fn restart_camera_last_detected_reset_timeout(&self) {
        let imp = self.imp();

        if let Some(source_id) = imp.camera_last_detected_reset_timeout.take() {
            source_id.remove();
        }

        let source_id = glib::timeout_add_local_once(
            CAMERA_LAST_DETECTED_RESET_DELAY,
            clone!(
                #[weak(rename_to = obj)]
                self,
                move || {
                    let imp = obj.imp();
                    imp.camera_last_detected_reset_timeout.replace(None);

                    imp.camera_last_detected.replace(None);
                },
            ),
        );
        imp.camera_last_detected_reset_timeout
            .replace(Some(source_id));
    }

    fn handle_code_detected(&self, code: &str) {
        let imp = self.imp();

        if imp
            .camera_last_detected
            .borrow()
            .as_ref()
            .is_some_and(|last_detected| last_detected == code)
        {
            return;
        }

        tracing::debug!("Detected code: {}", code);

        if let Some((id, data_fields)) = entity_from_qrcode(code) {
            self.emit_detected(&id, data_fields);
        } else {
            self.emit_by_name::<()>("detected-invalid", &[&code]);
        }

        imp.camera_last_detected.replace(Some(code.to_string()));
        self.restart_camera_last_detected_reset_timeout();
    }
}

fn entity_from_qrcode(code: &str) -> Option<(EntityId, Vec<EntityDataField>)> {
    entity_from_national_id(code)
        .or_else(|| entity_from_qrifying_cea(code))
        .or_else(|| entity_from_uets_qrcode_format(code))
}

fn entity_from_uets_qrcode_format(code: &str) -> Option<(EntityId, Vec<EntityDataField>)> {
    Some((code.strip_prefix("UETS:").map(EntityId::new)?, vec![]))
}

fn entity_from_qrifying_cea(code: &str) -> Option<(EntityId, Vec<EntityDataField>)> {
    let mut substrings = code.splitn(4, '_');
    let name = substrings.next()?;
    let student_id = substrings.next()?;
    let bpsu_email = substrings.next()?;
    let program = substrings.next()?;

    Some((
        EntityId::new(student_id),
        vec![
            EntityDataField::Kind(EntityKind::Person),
            EntityDataField::Name(name.to_string()),
            EntityDataField::Email(bpsu_email.to_string()),
            EntityDataField::Program(program.to_string()),
        ],
    ))
}

fn entity_from_national_id(code: &str) -> Option<(EntityId, Vec<EntityDataField>)> {
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

    let mut fields = vec![
        EntityDataField::Kind(EntityKind::Person),
        EntityDataField::Name(format!(
            "{}, {} {}",
            case::to_title_case(&data.subject.last_name),
            case::to_title_case(&data.subject.first_name),
            data.subject
                .middle_name
                .chars()
                .next()
                .map(|c| format!("{}.", c.to_uppercase()))
                .unwrap_or_default(),
        )),
    ];

    match data.subject.sex.parse::<Sex>() {
        Ok(sex) => fields.push(EntityDataField::Sex(sex)),
        Err(err) => tracing::warn!("Failed to parse sex: {:?}", err),
    }

    Some((EntityId::new(data.subject.pcn), fields))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uets_qrcode_format() {
        let code = "UETS: ABC123 ";
        let (id, data_fields) = entity_from_uets_qrcode_format(code).unwrap();
        assert_eq!(id.to_string(), " ABC123 ");
        assert!(data_fields.is_empty());

        let code = "UETS:";
        let (id, data_fields) = entity_from_uets_qrcode_format(code).unwrap();
        assert_eq!(id.to_string(), "");
        assert!(data_fields.is_empty());

        let code = "U";
        let out = entity_from_uets_qrcode_format(code);
        assert!(out.is_none());
    }
}
