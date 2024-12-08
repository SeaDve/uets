use adw::{prelude::*, subclass::prelude::*};
use gtk::glib::{self, clone, closure_local};

use crate::{
    date_time,
    date_time_range::DateTimeRange,
    entity_id::EntityId,
    limit_reached::{LabelExt, SettingsExt},
    ui::{
        camera_live_feed_dialog::CameraLiveFeedDialog,
        detected_wo_id_dialog::DetectedWoIdDialog,
        entity_photo_gallery_dialog::EntityPhotoGalleryDialog,
        information_row::InformationRow,
        receive_dialog::{InvalidFileExtension, ReceiveDialog},
        time_graph::TimeGraph,
    },
    Application,
};

mod imp {
    use std::sync::OnceLock;

    use glib::subclass::Signal;

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

            klass.install_action(
                "dashboard-view.show-camera-live-feed-dialog",
                None,
                move |obj, _, _| {
                    let dialog = CameraLiveFeedDialog::new();

                    let camera = Application::get().camera().clone();
                    dialog.set_camera(Some(camera));

                    dialog.present(Some(obj));
                },
            );
            klass.install_action(
                "dashboard-view.show-detected-wo-id-dialog",
                None,
                move |obj, _, _| {
                    let dialog = DetectedWoIdDialog::new();

                    let app = Application::get();
                    let list = app.detected_wo_id_list();
                    dialog.set_model(Some(list));

                    dialog.present(Some(obj));
                },
            );
            klass.install_action(
                "dashboard-view.show-entity-gallery-dialog",
                None,
                move |obj, _, _| {
                    let dialog = EntityPhotoGalleryDialog::new();

                    let app = Application::get();
                    let list = app.timeline().entity_list();
                    dialog.set_model(Some(list));

                    dialog.connect_show_entity_request(clone!(
                        #[weak]
                        obj,
                        move |_, id| {
                            obj.emit_by_name::<()>("show-entity-request", &[id]);
                        }
                    ));

                    dialog.present(Some(obj));
                },
            );
            klass.install_action_async(
                "dashboard-view.register-data",
                None,
                |obj, _, _| async move {
                    let app = Application::get();

                    let valid_file_extensions =
                        &[".xls", ".xlsx", ".xlsm", ".xlsb", ".xla", ".xlam", ".ods"];
                    match ReceiveDialog::receive(valid_file_extensions, Some(&obj)).await {
                        Ok((_, bytes)) => {
                            if let Err(err) =
                                app.timeline().register_data_from_workbook_bytes(&bytes)
                            {
                                tracing::error!("Failed to register data: {:?}", err);

                                app.add_message_toast("Failed to register data");
                            } else {
                                app.add_message_toast("Data registered");
                            }
                        }
                        Err(err) => {
                            if err.is::<InvalidFileExtension>() {
                                app.add_message_toast("Unknown file type");
                            } else {
                                app.add_message_toast("Failed to receive file");
                            }

                            tracing::error!("Failed to receive file: {:?}", err)
                        }
                    }
                },
            );
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

            app.settings().connect_limit_reached_changed(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_n_inside_label();
                }
            ));

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

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| {
                vec![Signal::builder("show-entity-request")
                    .param_types([EntityId::static_type()])
                    .build()]
            })
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

    pub fn connect_show_entity_request<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &EntityId) + 'static,
    {
        self.connect_closure(
            "show-entity-request",
            false,
            closure_local!(|obj: &Self, id: &EntityId| f(obj, id)),
        )
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

        let app = Application::get();

        let n_inside = app.timeline().n_inside();
        imp.n_inside_label
            .set_label_from_limit_reached(n_inside, app.settings());
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
                .map(|dt_boxed| date_time::format::fuzzy(dt_boxed.0))
                .unwrap_or_default(),
        );
    }

    fn update_last_exit_dt_row(&self) {
        let imp = self.imp();

        let last_exit_dt = Application::get().timeline().last_exit_dt();
        imp.last_exit_dt_row.set_value(
            last_exit_dt
                .map(|dt_boxed| date_time::format::fuzzy(dt_boxed.0))
                .unwrap_or_default(),
        );
    }
}
