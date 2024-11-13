use adw::{prelude::*, subclass::prelude::*};
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use futures_channel::oneshot;
use gtk::glib::{self, clone, closure};

use crate::{date_time_range::DateTimeRange, list_model_enum, ui::time_picker::TimePicker};

use super::time_picker::NaiveTimeBoxed;

mod imp {
    use std::cell::{OnceCell, RefCell};

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/date_time_window.ui")]
    pub struct DateTimeWindow {
        #[template_child]
        pub(super) range_kind_dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub(super) range_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) from_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) from_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) from_calendar: TemplateChild<gtk::Calendar>,
        #[template_child]
        pub(super) from_time_picker: TemplateChild<TimePicker>,
        #[template_child]
        pub(super) to_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) to_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) to_calendar: TemplateChild<gtk::Calendar>,
        #[template_child]
        pub(super) to_time_picker: TemplateChild<TimePicker>,

        pub(super) result_tx: RefCell<Option<oneshot::Sender<()>>>,
        pub(super) range_kind_dropdown_selected_item_id: OnceCell<glib::SignalHandlerId>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DateTimeWindow {
        const NAME: &'static str = "UetsDateTimeWindow";
        type Type = super::DateTimeWindow;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("date-time-window.cancel", None, move |obj, _, _| {
                let imp = obj.imp();

                let _ = imp.result_tx.take().unwrap();
            });
            klass.install_action("date-time-window.done", None, move |obj, _, _| {
                let imp = obj.imp();

                imp.result_tx.take().unwrap().send(()).unwrap();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DateTimeWindow {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            self.range_kind_dropdown
                .set_expression(Some(&gtk::ClosureExpression::new::<String>(
                    &[] as &[gtk::Expression],
                    closure!(|list_item: adw::EnumListItem| {
                        DateTimeRangeKind::try_from(list_item.value())
                            .unwrap()
                            .display()
                    }),
                )));
            self.range_kind_dropdown
                .set_model(Some(&DateTimeRangeKind::new_model()));
            let range_kind_dropdown_selected_item_notify_id = self
                .range_kind_dropdown
                .connect_selected_item_notify(clone!(
                    #[weak]
                    obj,
                    move |_| {
                        obj.update_state();
                    }
                ));
            self.range_kind_dropdown_selected_item_id
                .set(range_kind_dropdown_selected_item_notify_id)
                .unwrap();

            self.from_switch
                .bind_property("active", &*self.from_box, "sensitive")
                .sync_create()
                .build();
            self.to_switch
                .bind_property("active", &*self.to_box, "sensitive")
                .sync_create()
                .build();

            self.from_switch.connect_active_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.handle_range_changed();
                }
            ));
            self.to_switch.connect_active_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.handle_range_changed();
                }
            ));

            self.from_calendar.connect_year_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.handle_range_changed();
                }
            ));
            self.from_calendar.connect_month_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.handle_range_changed();
                }
            ));
            self.from_calendar.connect_day_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.handle_range_changed();
                }
            ));
            self.from_time_picker.connect_time_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.handle_range_changed();
                }
            ));

            self.to_calendar.connect_year_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.handle_range_changed();
                }
            ));
            self.to_calendar.connect_month_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.handle_range_changed();
                }
            ));
            self.to_calendar.connect_day_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.handle_range_changed();
                }
            ));
            self.to_time_picker.connect_time_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.handle_range_changed();
                }
            ));

            obj.update_state();
            obj.update_range_label();
            obj.update_done_action_enabled();
        }

        fn dispose(&self) {
            let _ = self.result_tx.take();
        }
    }

    impl WidgetImpl for DateTimeWindow {}
    impl WindowImpl for DateTimeWindow {}
    impl AdwWindowImpl for DateTimeWindow {}
}

glib::wrapper! {
    pub struct DateTimeWindow(ObjectSubclass<imp::DateTimeWindow>)
        @extends gtk::Widget, gtk::Window, adw::Window;
}

impl DateTimeWindow {
    pub async fn pick(
        initial_range: DateTimeRange,
        parent: &impl IsA<gtk::Widget>,
    ) -> Result<DateTimeRange, oneshot::Canceled> {
        let root = parent.root().map(|r| r.downcast::<gtk::Window>().unwrap());

        let this = glib::Object::builder::<Self>()
            .property("transient-for", root)
            .property("modal", true)
            .build();

        this.set_range(initial_range);

        let imp = this.imp();

        let (result_tx, result_rx) = oneshot::channel();
        imp.result_tx.replace(Some(result_tx));

        this.present();

        if let Err(err @ oneshot::Canceled) = result_rx.await {
            this.close();

            return Err(err);
        }

        let range = this.range();

        this.close();

        Ok(range)
    }

    fn range(&self) -> DateTimeRange {
        let imp = self.imp();

        DateTimeRange {
            start: get_dt(&imp.from_switch, &imp.from_calendar, &imp.from_time_picker),
            end: get_dt(&imp.to_switch, &imp.to_calendar, &imp.to_time_picker),
        }
    }

    fn set_range(&self, range: DateTimeRange) {
        let imp = self.imp();

        let _from_switch_guard = imp.from_switch.freeze_notify();
        let _to_switch_guard = imp.to_switch.freeze_notify();

        let _from_calendar_guard = imp.from_calendar.freeze_notify();
        let _to_calendar_guard = imp.to_calendar.freeze_notify();

        let _from_time_picker_guard = imp.from_time_picker.freeze_notify();
        let _to_time_picker_guard = imp.to_time_picker.freeze_notify();

        set_dt(
            &imp.from_switch,
            &imp.from_calendar,
            &imp.from_time_picker,
            range.start,
        );

        set_dt(
            &imp.to_switch,
            &imp.to_calendar,
            &imp.to_time_picker,
            range.end,
        );
    }

    fn selected_range_kind(&self) -> DateTimeRangeKind {
        let imp = self.imp();

        imp.range_kind_dropdown
            .selected_item()
            .map_or(DateTimeRangeKind::default(), |o| {
                let item = o.downcast::<adw::EnumListItem>().unwrap();
                DateTimeRangeKind::try_from(item.value()).unwrap()
            })
    }

    fn set_selected_range_kind_no_notify(&self, kind: DateTimeRangeKind) {
        let imp = self.imp();

        let selected_item_notify_id = imp.range_kind_dropdown_selected_item_id.get().unwrap();
        imp.range_kind_dropdown
            .block_signal(selected_item_notify_id);
        imp.range_kind_dropdown.set_selected(kind.position());
        imp.range_kind_dropdown
            .unblock_signal(selected_item_notify_id);
    }

    fn handle_range_changed(&self) {
        let range = self.range();
        let selected_range_kind = self.selected_range_kind();

        if selected_range_kind
            .to_range()
            .is_some_and(|r| !range.eq_ignore_subsec(&r))
        {
            self.set_selected_range_kind_no_notify(DateTimeRangeKind::Custom);
        } else if range.is_all_time() {
            self.set_selected_range_kind_no_notify(DateTimeRangeKind::AllTime);
        }

        self.update_range_label();
        self.update_done_action_enabled();
    }

    fn update_range_label(&self) {
        let imp = self.imp();

        let range = self.range();

        if range.is_empty() {
            imp.range_label.set_label("Invalid Range");
        } else {
            imp.range_label.set_label(&range.label_markup());
        }
    }

    fn update_done_action_enabled(&self) {
        self.action_set_enabled("date-time-window.done", !self.range().is_empty());
    }

    fn update_state(&self) {
        let range = self
            .selected_range_kind()
            .to_range()
            .unwrap_or_else(DateTimeRange::today);
        self.set_range(range);
    }
}

fn set_dt(
    switch: &gtk::Switch,
    calendar: &gtk::Calendar,
    time_picker: &TimePicker,
    dt: Option<NaiveDateTime>,
) {
    switch.set_active(dt.is_some());

    if let Some(dt) = dt {
        calendar.set_year(dt.year());
        calendar.set_month(dt.month0() as i32);
        calendar.set_day(dt.day() as i32);
        time_picker.set_time(NaiveTimeBoxed(dt.time()));
    }
}

fn get_dt(
    switch: &gtk::Switch,
    calendar: &gtk::Calendar,
    time_picker: &TimePicker,
) -> Option<NaiveDateTime> {
    if !switch.is_active() {
        return None;
    }

    let (year, month, day) = calendar.date().ymd();
    let date = NaiveDate::from_ymd_opt(year, month as u32, day as u32).unwrap();
    Some(NaiveDateTime::new(date, time_picker.time().0))
}

#[derive(Debug, Default, Clone, Copy, glib::Enum)]
#[enum_type(name = "UetsDateTimeRangeKind")]
enum DateTimeRangeKind {
    #[default]
    Custom,
    AllTime,
    Today,
    Yesterday,
    ThisWeek,
    ThisMonth,
    ThisYear,
}

list_model_enum!(DateTimeRangeKind);

impl DateTimeRangeKind {
    fn display(&self) -> &'static str {
        match self {
            Self::Custom => "Custom",
            Self::AllTime => "All Time",
            Self::Today => "Today",
            Self::Yesterday => "Yesterday",
            Self::ThisWeek => "This Week",
            Self::ThisMonth => "This Month",
            Self::ThisYear => "This Year",
        }
    }

    fn to_range(self) -> Option<DateTimeRange> {
        Some(match self {
            DateTimeRangeKind::Custom => return None,
            DateTimeRangeKind::AllTime => DateTimeRange::all_time(),
            DateTimeRangeKind::Today => DateTimeRange::today(),
            DateTimeRangeKind::Yesterday => DateTimeRange::yesterday(),
            DateTimeRangeKind::ThisWeek => DateTimeRange::this_week(),
            DateTimeRangeKind::ThisMonth => DateTimeRange::this_month(),
            DateTimeRangeKind::ThisYear => DateTimeRange::this_year(),
        })
    }
}
