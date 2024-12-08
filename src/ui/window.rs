use adw::{prelude::*, subclass::prelude::*};
use gtk::glib::{self, clone};

use crate::{
    entity_data::EntityDataFieldTy,
    ui::{
        dashboard_view::DashboardView, entities_view::EntitiesView, settings_view::SettingsView,
        stocks_view::StocksView, timeline_view::TimelineView,
    },
    Application,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToastId {
    Detected,
    LimitReached,
}

mod imp {
    use std::{cell::RefCell, collections::HashMap};

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

        pub(super) toasts: RefCell<HashMap<ToastId, adw::Toast>>,
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

            self.dashboard_view.connect_show_entity_request(clone!(
                #[weak]
                obj,
                move |_, entity_id| {
                    let imp = obj.imp();
                    imp.entities_view.show_entity(entity_id);
                    imp.view_stack.set_visible_child_name("entities");
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

            match rppal::system::DeviceInfo::new() {
                Ok(device_info) => {
                    tracing::debug!("Running on {}", device_info.model());

                    obj.fullscreen();
                }
                Err(err) => {
                    tracing::warn!("Failed to get device info: {:?}", err);
                }
            }
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

    pub fn add_message_toast(&self, message: &str) {
        let imp = self.imp();

        imp.toast_overlay.add_toast(adw::Toast::new(message));
    }

    pub fn add_message_toast_with_id(&self, id: ToastId, message: &str) {
        let imp = self.imp();

        if let Some(toast) = imp.toasts.borrow().get(&id) {
            toast.set_title(message);
            imp.toast_overlay.add_toast(toast.clone());
            return;
        }

        let toast = adw::Toast::new(message);

        toast.connect_dismissed(clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| {
                let imp = obj.imp();
                imp.toasts.borrow_mut().remove(&id);
            }
        ));

        imp.toast_overlay.add_toast(toast.clone());

        imp.toasts.borrow_mut().insert(id, toast);
    }

    pub fn remove_message_toast_with_id(&self, id: ToastId) {
        let imp = self.imp();

        let toast = imp.toasts.borrow_mut().remove(&id);
        if let Some(toast) = toast {
            toast.dismiss();
        }
    }

    fn update_stocks_entities_stack_pages_display(&self) {
        let imp = self.imp();

        let mode = Application::get().settings().operation_mode();

        imp.entities_stack_page
            .set_icon_name(Some(mode.entities_view_icon_name()));

        imp.stocks_stack_page
            .set_visible(mode.is_valid_entity_data_field_ty(EntityDataFieldTy::StockId));
        imp.stocks_stack_page
            .set_icon_name(mode.stocks_view_icon_name());
    }
}
