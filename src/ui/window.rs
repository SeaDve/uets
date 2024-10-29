use adw::subclass::prelude::*;
use gtk::glib::{self, clone};

use crate::{
    settings::OperationMode,
    ui::{
        dashboard_view::DashboardView, entities_view::EntitiesView, settings_view::SettingsView,
        timeline_view::TimelineView,
    },
    Application,
};

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/window.ui")]
    pub struct Window {
        #[template_child]
        pub(super) dashboard_view: TemplateChild<DashboardView>,
        #[template_child]
        pub(super) assets_stack_page: TemplateChild<adw::ViewStackPage>,
        #[template_child]
        pub(super) entities_view: TemplateChild<EntitiesView>,
        #[template_child]
        pub(super) timeline_view: TemplateChild<TimelineView>,
        #[template_child]
        pub(super) settings_view: TemplateChild<SettingsView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "UetsWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Window {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            let app = Application::get();

            app.settings().connect_operation_mode_changed(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_entities_stack_page_display();
                }
            ));

            self.entities_view
                .bind_entity_list(app.timeline().entity_list());
            self.timeline_view.bind_timeline(app.timeline());

            obj.update_entities_stack_page_display();
        }
    }

    impl WidgetImpl for Window {}
    impl WindowImpl for Window {}
    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow;
}

impl Window {
    pub fn new(application: &Application) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }

    fn update_entities_stack_page_display(&self) {
        let imp = self.imp();

        let title = match Application::get().settings().operation_mode() {
            OperationMode::Counter | OperationMode::Attendance => "Entities",
            OperationMode::Inventory | OperationMode::Refrigerator => "Stocks",
        };
        imp.assets_stack_page.set_title(Some(title));

        let icon_name = match Application::get().settings().operation_mode() {
            OperationMode::Counter | OperationMode::Attendance => "people-symbolic",
            OperationMode::Inventory => "preferences-desktop-apps-symbolic",
            OperationMode::Refrigerator => "egg-symbolic",
        };
        imp.assets_stack_page.set_icon_name(Some(icon_name));
    }
}
