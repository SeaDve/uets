use adw::subclass::prelude::*;
use gtk::glib::{self, clone};

use crate::{
    settings::OperationMode,
    ui::{
        dashboard_view::DashboardView, entities_view::EntitiesView, settings_view::SettingsView,
        stocks_view::StocksView, timeline_view::TimelineView,
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
        pub(super) assets_view_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) entities_view: TemplateChild<EntitiesView>,
        #[template_child]
        pub(super) stocks_view: TemplateChild<StocksView>,
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
            let timeline = app.timeline();

            app.settings().connect_operation_mode_changed(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_entities_stack_page_display();
                    obj.update_assets_view_stack();
                }
            ));

            self.entities_view.bind_entity_list(timeline.entity_list());
            self.stocks_view.bind_stock_list(timeline.stock_list());
            self.timeline_view.bind_timeline(timeline);

            obj.update_entities_stack_page_display();
            obj.update_assets_view_stack();
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

    fn update_assets_view_stack(&self) {
        let imp = self.imp();

        match Application::get().settings().operation_mode() {
            OperationMode::Counter | OperationMode::Attendance => {
                imp.assets_view_stack.set_visible_child(&*imp.entities_view);
            }
            OperationMode::Inventory | OperationMode::Refrigerator => {
                imp.assets_view_stack.set_visible_child(&*imp.stocks_view);
            }
        }
    }
}
