use std::cmp::Ordering;

use gtk::glib::{self, prelude::*};

#[macro_export]
macro_rules! list_model_enum {
    ($name:ident) => {
        impl $name {
            fn new_model() -> adw::EnumListModel {
                adw::EnumListModel::new($name::static_type())
            }

            fn model_position(&self) -> u32 {
                *self as u32
            }
        }

        impl TryFrom<i32> for $name {
            type Error = i32;

            fn try_from(val: i32) -> Result<Self, Self::Error> {
                use gtk::glib::translate::TryFromGlib;

                unsafe { Self::try_from_glib(val) }
            }
        }
    };
}

pub fn new_sorter<T: IsA<glib::Object>>(
    is_reverse: bool,
    predicate: impl Fn(&T, &T) -> Ordering + 'static,
) -> gtk::CustomSorter {
    if is_reverse {
        gtk::CustomSorter::new(move |a, b| {
            let a = a.downcast_ref::<T>().unwrap();
            let b = b.downcast_ref::<T>().unwrap();
            predicate(a, b).reverse().into()
        })
    } else {
        gtk::CustomSorter::new(move |a, b| {
            let a = a.downcast_ref::<T>().unwrap();
            let b = b.downcast_ref::<T>().unwrap();
            predicate(a, b).into()
        })
    }
}

pub fn new_filter<T: IsA<glib::Object>>(
    predicate: impl Fn(&T) -> bool + 'static,
) -> gtk::CustomFilter {
    gtk::CustomFilter::new(move |o| {
        let item = o.downcast_ref::<T>().unwrap();
        predicate(item)
    })
}
