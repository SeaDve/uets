use adw::subclass::prelude::*;
use gtk::glib;

use crate::{
    application::Application,
    ui::{dashboard_view::DashboardView, settings_view::SettingsView, timeline_view::TimelineView},
};

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/window.ui")]
    pub struct Window {
        #[template_child]
        pub(super) dashboard_view: TemplateChild<DashboardView>,
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

            self.timeline_view
                .bind_timeline(Application::get().timeline());
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
}
