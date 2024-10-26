use gtk::{
    glib::{self, clone},
    subclass::prelude::*,
};

use crate::Application;

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/dashboard_view.ui")]
    pub struct DashboardView {
        #[template_child]
        pub(super) page: TemplateChild<adw::PreferencesPage>, // Unused
        #[template_child]
        pub(super) n_inside_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DashboardView {
        const NAME: &'static str = "UetsDashboardView";
        type Type = super::DashboardView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DashboardView {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            Application::get()
                .entity_tracker()
                .connect_n_inside_notify(clone!(
                    #[weak]
                    obj,
                    move |_| {
                        obj.update_n_inside_label();
                    }
                ));

            obj.update_n_inside_label();
        }

        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for DashboardView {}
}

glib::wrapper! {
    pub struct DashboardView(ObjectSubclass<imp::DashboardView>)
        @extends gtk::Widget;
}

impl DashboardView {
    pub fn new() -> Self {
        glib::Object::new()
    }

    fn update_n_inside_label(&self) {
        let imp = self.imp();

        let n_inside = Application::get().entity_tracker().n_inside();
        imp.n_inside_label.set_text(&n_inside.to_string());
    }
}
