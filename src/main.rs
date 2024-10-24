#![allow(clippy::new_without_default)]
#![warn(rust_2018_idioms, clippy::unused_async, clippy::dbg_macro)]

mod application;
mod detector;
mod entity;
mod entity_id;
mod tracker;
mod ui;

use gtk::{glib, prelude::*};

use self::application::Application;

const APP_ID: &str = "io.github.seadve.Uets";

fn main() -> glib::ExitCode {
    tracing_subscriber::fmt::init();

    let app = Application::new();
    app.run()
}
