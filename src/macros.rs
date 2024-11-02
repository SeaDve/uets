#[macro_export]
macro_rules! list_model_enum {
    ($name:ident) => {
        #[allow(unused)]
        impl $name {
            fn new_model() -> adw::EnumListModel {
                adw::EnumListModel::new($name::static_type())
            }

            fn position(&self) -> u32 {
                *self as u32
            }
        }

        impl TryFrom<i32> for $name {
            type Error = i32;

            fn try_from(val: i32) -> Result<Self, Self::Error> {
                unsafe { Self::try_from_glib(val) }
            }
        }
    };
}
