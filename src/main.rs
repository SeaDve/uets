#![allow(clippy::new_without_default)]
#![warn(rust_2018_idioms, clippy::unused_async, clippy::dbg_macro)]

mod application;
mod date_time;
mod db;
mod detector;
mod entity;
mod entity_data;
mod entity_id;
mod entity_kind;
mod entity_tracker;
mod settings;
mod timeline;
mod timeline_item;
mod ui;

use std::path::Path;

use gtk::{gio, glib, prelude::*};

use self::application::Application;

const APP_ID: &str = "io.github.seadve.Uets";
const GRESOURCE_PREFIX: &str = "/io/github/seadve/Uets/";

fn main() -> glib::ExitCode {
    tracing_subscriber::fmt::init();

    let data = gvdb::gresource::BundleBuilder::from_directory(
        GRESOURCE_PREFIX,
        Path::new("data/resources/"),
        true,
        true,
    )
    .unwrap()
    .build()
    .unwrap();
    let resource = gio::Resource::from_data(&glib::Bytes::from_owned(data)).unwrap();
    gio::resources_register(&resource);

    let app = Application::new();
    app.run()
}
