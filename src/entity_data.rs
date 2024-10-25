#![allow(unused)]

use gtk::glib;

#[derive(Debug)]
pub struct InventoryEntityData {
    name: String,
    location: String,
    expiration_date: glib::DateTime,
}

#[derive(Debug)]
pub struct RefrigeratorEntityData {
    name: String,
    expiration_date: glib::DateTime,
}

#[derive(Debug)]
pub struct AttendanceEntityData {
    name: String,
}

#[derive(Debug)]
pub enum EntityData {
    Inventory(InventoryEntityData),
    Refrigerator(RefrigeratorEntityData),
    Attendance(AttendanceEntityData),
}
