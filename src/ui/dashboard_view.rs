use gtk::{
    glib::{self, clone},
    subclass::prelude::*,
};

use crate::{ui::information_row::InformationRow, Application};

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/dashboard_view.ui")]
    pub struct DashboardView {
        #[template_child]
        pub(super) page: TemplateChild<adw::PreferencesPage>, // Unused
        #[template_child]
        pub(super) n_inside_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) last_entry_dt_row: TemplateChild<InformationRow>,
        #[template_child]
        pub(super) last_exit_dt_row: TemplateChild<InformationRow>,
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

            let app = Application::get();

            app.entity_tracker().connect_n_inside_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_n_inside_label();
                }
            ));

            app.entity_tracker()
                .timeline()
                .connect_last_entry_dt_notify(clone!(
                    #[weak]
                    obj,
                    move |_| {
                        obj.update_last_entry_dt_row();
                    }
                ));
            app.entity_tracker()
                .timeline()
                .connect_last_exit_dt_notify(clone!(
                    #[weak]
                    obj,
                    move |_| {
                        obj.update_last_exit_dt_row();
                    }
                ));

            obj.update_n_inside_label();
            obj.update_last_entry_dt_row();
            obj.update_last_exit_dt_row();
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

    fn update_last_entry_dt_row(&self) {
        let imp = self.imp();

        let last_entry_dt = Application::get()
            .entity_tracker()
            .timeline()
            .last_entry_dt();
        imp.last_entry_dt_row.set_value(
            last_entry_dt
                .map(|dt| dt.local_fuzzy_display())
                .unwrap_or_default(),
        );
    }

    fn update_last_exit_dt_row(&self) {
        let imp = self.imp();

        let last_exit_dt = Application::get()
            .entity_tracker()
            .timeline()
            .last_exit_dt();
        imp.last_exit_dt_row.set_value(
            last_exit_dt
                .map(|dt| dt.local_fuzzy_display())
                .unwrap_or_default(),
        );
    }
}
