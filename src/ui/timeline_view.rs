use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    entity_id::EntityId, stock_id::StockId, timeline::Timeline, ui::timeline_row::TimelineRow,
};

mod imp {
    use std::{cell::Cell, sync::OnceLock};

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
        pub(super) main_page: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub(super) scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub(super) list_view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub(super) scroll_to_bottom_revealer: TemplateChild<gtk::Revealer>,

        pub(super) is_sticky: Cell<bool>,
        pub(super) is_auto_scrolling: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TimelineView {
        const NAME: &'static str = "UetsTimelineView";
        type Type = super::TimelineView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("timeline-view.scroll-to-bottom", None, move |obj, _, _| {
                obj.scroll_to_bottom();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TimelineView {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            let vadj = self.list_view.vadjustment().unwrap();
            vadj.connect_value_changed(clone!(
                #[weak]
                obj,
                move |_| {
                    let imp = obj.imp();

                    let is_at_bottom = obj.is_at_bottom();
                    if imp.is_auto_scrolling.get() {
                        if is_at_bottom {
                            imp.is_auto_scrolling.set(false);
                            imp.is_sticky.set(true);
                        } else {
                            obj.scroll_to_bottom();
                        }
                    } else {
                        imp.is_sticky.set(is_at_bottom);
                    }

                    obj.update_scroll_to_bottom_revealer_reveal_child();
                }
            ));
            vadj.connect_upper_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    let imp = obj.imp();

                    if imp.is_sticky.get() {
                        obj.scroll_to_bottom();
                    }

                    obj.update_scroll_to_bottom_revealer_reveal_child();
                }
            ));
            vadj.connect_page_size_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    let imp = obj.imp();

                    if imp.is_sticky.get() {
                        obj.scroll_to_bottom();
                    }

                    obj.update_scroll_to_bottom_revealer_reveal_child();
                }
            ));

            self.scroll_to_bottom_revealer
                .connect_child_revealed_notify(clone!(
                    #[weak]
                    obj,
                    move |_| {
                        obj.update_scroll_to_bottom_revealer_can_target();
                    }
                ));

            let factory = gtk::SignalListItemFactory::new();
            factory.connect_setup(clone!(
                #[weak]
                obj,
                move |_, list_item| {
                    let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
                    list_item.set_selectable(false);
                    list_item.set_activatable(false);

                    let row = TimelineRow::new();
                    row.connect_show_entity_request(clone!(
                        #[weak]
                        obj,
                        move |_, id| {
                            obj.emit_by_name::<()>("show-entity-request", &[&id]);
                        }
                    ));
                    row.connect_show_stock_request(clone!(
                        #[weak]
                        obj,
                        move |_, id| {
                            obj.emit_by_name::<()>("show-stock-request", &[&id]);
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
                    Signal::builder("show-entity-request")
                        .param_types([EntityId::static_type()])
                        .build(),
                    Signal::builder("show-stock-request")
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

    pub fn connect_show_stock_request<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &StockId) + 'static,
    {
        self.connect_closure(
            "show-stock-request",
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

    fn scroll_to_bottom(&self) {
        let imp = self.imp();
        imp.is_auto_scrolling.set(true);
        imp.scrolled_window
            .emit_scroll_child(gtk::ScrollType::End, false);
    }

    fn is_at_bottom(&self) -> bool {
        let imp = self.imp();
        let vadj = imp.list_view.vadjustment().unwrap();
        vadj.value() + vadj.page_size() == vadj.upper()
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

    fn update_scroll_to_bottom_revealer_reveal_child(&self) {
        let imp = self.imp();

        imp.scroll_to_bottom_revealer
            .set_reveal_child(!self.is_at_bottom());
    }

    fn update_scroll_to_bottom_revealer_can_target(&self) {
        let imp = self.imp();

        imp.scroll_to_bottom_revealer
            .set_can_target(imp.scroll_to_bottom_revealer.is_child_revealed());
    }
}
