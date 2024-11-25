#![allow(clippy::new_without_default, clippy::type_complexity)]
#![warn(
    rust_2018_idioms,
    clippy::unused_async,
    clippy::dbg_macro,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::todo
)]

mod application;
mod colors;
mod config;
mod date_time_boxed;
mod date_time_range;
mod db;
mod detector;
mod entity;
mod entity_data;
mod entity_data_index;
mod entity_id;
mod entity_list;
mod format;
mod fuzzy_filter;
mod fuzzy_sorter;
mod log;
mod report;
mod report_table;
mod search_query;
mod search_query_ext;
mod settings;
mod stock;
mod stock_id;
mod stock_list;
mod time_graph;
mod timeline;
mod timeline_item;
mod timeline_item_kind;
mod ui;
mod utils;

use std::path::Path;

use gtk::{gio, glib, prelude::*};

use self::application::Application;

const APP_ID: &str = "io.github.seadve.Uets";
const GRESOURCE_PREFIX: &str = "/io/github/seadve/Uets/";

fn main() -> glib::ExitCode {
    tracing_subscriber::fmt::init();

    gst::init().unwrap();
    gstgtk4::plugin_register_static().unwrap();

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
