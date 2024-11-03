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
        pub(super) toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub(super) view_stack: TemplateChild<adw::ViewStack>,
        #[template_child]
        pub(super) dashboard_view: TemplateChild<DashboardView>,
        #[template_child]
        pub(super) stocks_stack_page: TemplateChild<adw::ViewStackPage>,
        #[template_child]
        pub(super) stocks_view: TemplateChild<StocksView>,
        #[template_child]
        pub(super) entities_stack_page: TemplateChild<adw::ViewStackPage>,
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
            let timeline = app.timeline();

            app.settings().connect_operation_mode_changed(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_stocks_entities_stack_pages_display();
                }
            ));

            self.stocks_view.connect_show_timeline_request(clone!(
                #[weak]
                obj,
                move |_, stock_id| {
                    let imp = obj.imp();
                    imp.timeline_view.show_stock(stock_id);
                    imp.view_stack.set_visible_child_name("timeline");
                }
            ));
            self.stocks_view.connect_show_entities_request(clone!(
                #[weak]
                obj,
                move |_, stock_id| {
                    let imp = obj.imp();
                    imp.entities_view.show_entities_with_stock_id(stock_id);
                    imp.view_stack.set_visible_child_name("entities");
                }
            ));

            self.entities_view.connect_show_stock_request(clone!(
                #[weak]
                obj,
                move |_, stock_id| {
                    let imp = obj.imp();
                    imp.stocks_view.show_stock(stock_id);
                    imp.view_stack.set_visible_child_name("stocks");
                }
            ));
            self.entities_view.connect_show_timeline_request(clone!(
                #[weak]
                obj,
                move |_, entity_id| {
                    let imp = obj.imp();
                    imp.timeline_view.show_entity(entity_id);
                    imp.view_stack.set_visible_child_name("timeline");
                }
            ));

            self.timeline_view.connect_show_entity_request(clone!(
                #[weak]
                obj,
                move |_, entity_id| {
                    let imp = obj.imp();
                    imp.entities_view.show_entity(entity_id);
                    imp.view_stack.set_visible_child_name("entities");
                }
            ));
            self.timeline_view.connect_show_stock_request(clone!(
                #[weak]
                obj,
                move |_, stock_id| {
                    let imp = obj.imp();
                    imp.stocks_view.show_stock(stock_id);
                    imp.view_stack.set_visible_child_name("stocks");
                }
            ));

            self.entities_view.bind_entity_list(timeline.entity_list());
            self.stocks_view.bind_stock_list(timeline.stock_list());
            self.timeline_view.bind_timeline(timeline);

            obj.update_stocks_entities_stack_pages_display();
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

    pub fn add_toast(&self, toast: adw::Toast) {
        self.imp().toast_overlay.add_toast(toast);
    }

    fn update_stocks_entities_stack_pages_display(&self) {
        let imp = self.imp();

        let operation_mode = Application::get().settings().operation_mode();

        match operation_mode {
            OperationMode::Counter | OperationMode::Attendance => {
                imp.stocks_stack_page.set_visible(false);

                imp.entities_stack_page
                    .set_icon_name(Some("people-symbolic"));
            }
            OperationMode::Inventory | OperationMode::Refrigerator => {
                imp.stocks_stack_page.set_visible(true);

                match operation_mode {
                    OperationMode::Inventory => {
                        imp.stocks_stack_page
                            .set_icon_name(Some("preferences-desktop-apps-symbolic"));
                    }
                    OperationMode::Refrigerator => {
                        imp.stocks_stack_page.set_icon_name(Some("egg-symbolic"));
                    }
                    _ => unreachable!(),
                }

                imp.entities_stack_page
                    .set_icon_name(Some("tag-outline-symbolic"));
            }
        };
    }
}
