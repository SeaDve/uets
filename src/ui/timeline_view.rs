use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    entity_id::EntityId, stock_id::StockId, timeline::Timeline, ui::timeline_row::TimelineRow,
};

mod imp {
    use std::sync::OnceLock;

    use glib::subclass::Signal;

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/timeline_view.ui")]
    pub struct TimelineView {
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) no_data_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub(super) main_page: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub(super) list_view: TemplateChild<gtk::ListView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TimelineView {
        const NAME: &'static str = "UetsTimelineView";
        type Type = super::TimelineView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TimelineView {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            let factory = gtk::SignalListItemFactory::new();
            factory.connect_setup(clone!(
                #[weak]
                obj,
                move |_, list_item| {
                    let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
                    list_item.set_selectable(false);
                    list_item.set_activatable(false);

                    let row = TimelineRow::new();
                    row.connect_show_entity(clone!(
                        #[weak]
                        obj,
                        move |_, id| {
                            obj.emit_by_name::<()>("show-entity", &[&id]);
                        }
                    ));
                    row.connect_show_stock(clone!(
                        #[weak]
                        obj,
                        move |_, id| {
                            obj.emit_by_name::<()>("show-stock", &[&id]);
                        }
                    ));

                    list_item
                        .property_expression("item")
                        .bind(&row, "item", glib::Object::NONE);

                    list_item.set_child(Some(&row));
                }
            ));
            self.list_view.set_factory(Some(&factory));
        }

        fn dispose(&self) {
            self.dispose_template();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("show-entity")
                        .param_types([EntityId::static_type()])
                        .build(),
                    Signal::builder("show-stock")
                        .param_types([StockId::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for TimelineView {}
}

glib::wrapper! {
    pub struct TimelineView(ObjectSubclass<imp::TimelineView>)
        @extends gtk::Widget;
}

impl TimelineView {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn connect_show_entity<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &EntityId) + 'static,
    {
        self.connect_closure(
            "show-entity",
            false,
            closure_local!(|obj: &Self, id: &EntityId| f(obj, id)),
        )
    }

    pub fn connect_show_stock<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &StockId) + 'static,
    {
        self.connect_closure(
            "show-stock",
            false,
            closure_local!(|obj: &Self, id: &StockId| f(obj, id)),
        )
    }

    pub fn bind_timeline(&self, timeline: &Timeline) {
        let imp = self.imp();

        timeline.connect_items_changed(clone!(
            #[weak(rename_to = obj)]
            self,
            move |_, _, _, _| {
                obj.update_stack();
            }
        ));

        let selection_model = gtk::NoSelection::new(Some(timeline.clone()));
        imp.list_view.set_model(Some(&selection_model));

        self.update_stack();
    }

    fn update_stack(&self) {
        let imp = self.imp();

        let selection_model = imp
            .list_view
            .model()
            .unwrap()
            .downcast::<gtk::NoSelection>()
            .unwrap();
        let timeline = selection_model
            .model()
            .unwrap()
            .downcast::<Timeline>()
            .unwrap();

        if timeline.is_empty() {
            imp.stack.set_visible_child(&*imp.no_data_page);
        } else {
            imp.stack.set_visible_child(&*imp.main_page);
        }
    }
}
