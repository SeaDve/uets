use std::time::Duration;

use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};
use serde::{Deserialize, Serialize};

use crate::{camera::Camera, entity_data::EntityData, entity_id::EntityId};

mod imp {
    use std::sync::OnceLock;

    use gtk::glib::subclass::Signal;

    use super::*;

    #[derive(Default)]
    pub struct Detector {
        pub(super) camera: Camera,
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

            glib::spawn_future_local(clone!(
                #[weak]
                obj,
                async move {
                    let imp = obj.imp();

                    loop {
                        if !imp.camera.has_started() {
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
                    if let Some((id, data)) = entity_from_qrcode(qrcode) {
                        obj.emit_detected(&id, Some(&data));
                    } else {
                        tracing::warn!("Invalid entity code: {}", qrcode);
                    }
                }
            ));
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
}

impl Default for Detector {
    fn default() -> Self {
        Self::new()
    }
}

fn entity_from_qrcode(code: &str) -> Option<(EntityId, EntityData)> {
    entity_from_national_id(code).or_else(|| entity_from_qrfying_ncea(code))
}

#[allow(unused)]
fn entity_from_qrfying_ncea(code: &str) -> Option<(EntityId, EntityData)> {
    let mut substrings = code.splitn(4, '_');
    let name = substrings.next()?;
    let student_id = substrings.next()?;
    let bpsu_email = substrings.next()?;
    let program = substrings.next()?;
    Some((EntityId::new(student_id), EntityData::new()))
}

fn entity_from_national_id(code: &str) -> Option<(EntityId, EntityData)> {
    #[derive(Serialize, Deserialize)]
    pub struct Subject {
        last_name: String,
        first_name: String,
        middle_name: String,
        sex: String,
        date_of_birth: String,
        place_of_birth: String,
        pcn: String,
    }

    #[derive(Serialize, Deserialize)]
    pub struct Data {
        date_issued: String,
        issuer: String,
        subject: Subject,
    }

    let data = serde_json::from_str::<Data>(code)
        .inspect_err(|err| tracing::debug!("Failed to deserialize national id data: {:?}", err))
        .ok()?;

    Some((EntityId::new(data.subject.pcn), EntityData::new()))
}
