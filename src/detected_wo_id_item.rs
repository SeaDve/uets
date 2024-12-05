use chrono::{DateTime, Utc};
use gtk::{glib, subclass::prelude::*};

use crate::{db, jpeg_image::JpegImage};

mod imp {
    use std::cell::OnceCell;

    use super::*;

    #[derive(Default)]
    pub struct DetectedWoIdItem {
        pub(super) dt: OnceCell<DateTime<Utc>>,
        pub(super) image: OnceCell<Option<JpegImage>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DetectedWoIdItem {
        const NAME: &'static str = "UetsDetectedWoIdItem";
        type Type = super::DetectedWoIdItem;
    }

    impl ObjectImpl for DetectedWoIdItem {}
}

glib::wrapper! {
    pub struct DetectedWoIdItem(ObjectSubclass<imp::DetectedWoIdItem>);
}

impl DetectedWoIdItem {
    pub fn new(dt: DateTime<Utc>, image: Option<JpegImage>) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.dt.set(dt).unwrap();
        imp.image.set(image).unwrap();

        this
    }

    pub fn from_db(dt: DateTime<Utc>, raw: db::RawDetectedWoIdItem) -> Self {
        Self::new(dt, raw.image)
    }

    pub fn to_db(&self) -> db::RawDetectedWoIdItem {
        db::RawDetectedWoIdItem {
            image: self.image().clone(),
        }
    }

    pub fn dt(&self) -> DateTime<Utc> {
        *self.imp().dt.get().unwrap()
    }

    pub fn image(&self) -> Option<JpegImage> {
        self.imp().image.get().unwrap().clone()
    }
}
