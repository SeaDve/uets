#![allow(unused)]

use gtk::glib;

use crate::date_time::DateTime;

#[derive(Debug)]
pub struct InventoryEntityData {
    name: String,
    location: String,
    expiration_date: DateTime,
}

#[derive(Debug)]
pub struct RefrigeratorEntityData {
    name: String,
    expiration_date: DateTime,
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
