use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    date_time_range::DateTimeRange,
    format,
    ui::{information_row::InformationRow, time_graph::TimeGraph},
    Application,
};

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
        pub(super) max_n_inside_row: TemplateChild<InformationRow>,
        #[template_child]
        pub(super) n_entries_row: TemplateChild<InformationRow>,
        #[template_child]
        pub(super) n_exits_row: TemplateChild<InformationRow>,
        #[template_child]
        pub(super) last_entry_dt_row: TemplateChild<InformationRow>,
        #[template_child]
        pub(super) last_exit_dt_row: TemplateChild<InformationRow>,
        #[template_child]
        pub(super) n_inside_graph: TemplateChild<TimeGraph>,
        #[template_child]
        pub(super) max_n_inside_graph: TemplateChild<TimeGraph>,
        #[template_child]
        pub(super) n_entries_graph: TemplateChild<TimeGraph>,
        #[template_child]
        pub(super) n_exits_graph: TemplateChild<TimeGraph>,
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
            let timeline = app.timeline();

            timeline.connect_items_changed(clone!(
                #[weak]
                obj,
                move |_, _, _, _| {
                    obj.update_graphs_data();
                }
            ));
            timeline.connect_n_inside_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_n_inside_label();
                }
            ));
            timeline.connect_max_n_inside_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_max_n_inside_row();
                }
            ));
            timeline.connect_n_entries_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_n_entries_label();
                }
            ));
            timeline.connect_n_exits_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_n_exits_label();
                }
            ));
            timeline.connect_last_entry_dt_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_last_entry_dt_row();
                }
            ));
            timeline.connect_last_exit_dt_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_last_exit_dt_row();
                }
            ));

            obj.update_graphs_data();
            obj.update_n_inside_label();
            obj.update_max_n_inside_row();
            obj.update_n_entries_label();
            obj.update_n_exits_label();
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

    fn update_graphs_data(&self) {
        let imp = self.imp();

        let app = Application::get();
        let timeline = app.timeline();

        let data = timeline
            .iter(&DateTimeRange::all_time())
            .map(|item| (item.dt(), timeline.n_inside_for_dt(item.dt())))
            .collect::<Vec<_>>();
        imp.n_inside_graph.set_data(data);

        let data = timeline
            .iter(&DateTimeRange::all_time())
            .map(|item| (item.dt(), timeline.max_n_inside_for_dt(item.dt())))
            .collect::<Vec<_>>();
        imp.max_n_inside_graph.set_data(data);

        let data = timeline
            .iter(&DateTimeRange::all_time())
            .map(|item| (item.dt(), timeline.n_entries_for_dt(item.dt())))
            .collect::<Vec<_>>();
        imp.n_entries_graph.set_data(data);

        let data = timeline
            .iter(&DateTimeRange::all_time())
            .map(|item| (item.dt(), timeline.n_exits_for_dt(item.dt())))
            .collect::<Vec<_>>();
        imp.n_exits_graph.set_data(data);
    }

    fn update_n_inside_label(&self) {
        let imp = self.imp();

        let n_inside = Application::get().timeline().n_inside();
        imp.n_inside_label.set_label(&n_inside.to_string());
    }

    fn update_max_n_inside_row(&self) {
        let imp = self.imp();

        let max_n_inside = Application::get().timeline().max_n_inside();
        imp.max_n_inside_row.set_value(max_n_inside.to_string());
    }

    fn update_n_entries_label(&self) {
        let imp = self.imp();

        let n_entries = Application::get().timeline().n_entries();
        imp.n_entries_row.set_value(n_entries.to_string());
    }

    fn update_n_exits_label(&self) {
        let imp = self.imp();

        let n_exits = Application::get().timeline().n_exits();
        imp.n_exits_row.set_value(n_exits.to_string());
    }

    fn update_last_entry_dt_row(&self) {
        let imp = self.imp();

        let last_entry_dt = Application::get().timeline().last_entry_dt();
        imp.last_entry_dt_row.set_value(
            last_entry_dt
                .map(|dt_boxed| format::fuzzy_dt(dt_boxed.0))
                .unwrap_or_default(),
        );
    }

    fn update_last_exit_dt_row(&self) {
        let imp = self.imp();

        let last_exit_dt = Application::get().timeline().last_exit_dt();
        imp.last_exit_dt_row.set_value(
            last_exit_dt
                .map(|dt_boxed| format::fuzzy_dt(dt_boxed.0))
                .unwrap_or_default(),
        );
    }
}
