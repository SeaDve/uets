use std::ops::Range;

use adw::{prelude::*, subclass::prelude::*};
use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Weekday};
use futures_channel::oneshot;
use gtk::glib::{self, clone, closure};

use crate::{list_model_enum, ui::time_picker::TimePicker};

use super::time_picker::NaiveTimeBoxed;

const DT_FORMAT: &str = "%b %-d %Y %r";

const WEEK_START: Weekday = Weekday::Sun;

const MIN_TIME: NaiveTime = NaiveTime::MIN;

#[allow(deprecated)]
const MAX_TIME: NaiveTime = NaiveTime::from_hms(23, 59, 59);

#[derive(Debug, Default, Clone, Copy, glib::Enum)]
#[enum_type(name = "UetsDateTimeRangeKind")]
enum DateTimeRangeKind {
    #[default]
    Custom,
    AllTime,
    Today,
    Yesterday,
    ThisWeek,
    LastWeek,
    ThisMonth,
    LastMonth,
    ThisYear,
    LastYear,
}

list_model_enum!(DateTimeRangeKind);

impl DateTimeRangeKind {
    fn display(&self) -> &'static str {
        match self {
            Self::AllTime => "All Time",
            Self::Custom => "Custom",
            Self::Today => "Today",
            Self::Yesterday => "Yesterday",
            Self::ThisWeek => "This Week",
            Self::LastWeek => "Last Week",
            Self::ThisMonth => "This Month",
            Self::LastMonth => "Last Month",
            Self::ThisYear => "This Year",
            Self::LastYear => "Last Year",
        }
    }

    fn range(&self) -> Option<Range<NaiveDateTime>> {
        let now = Local::now().naive_local();

        let ret = match self {
            Self::AllTime | Self::Custom => {
                return None;
            }
            Self::Today => {
                NaiveDateTime::new(now.date(), MIN_TIME)..NaiveDateTime::new(now.date(), MAX_TIME)
            }
            Self::Yesterday => {
                let yesterday = now.date().pred_opt().unwrap();
                NaiveDateTime::new(yesterday, MIN_TIME)..NaiveDateTime::new(yesterday, MAX_TIME)
            }
            Self::ThisWeek => {
                let today = now.date();

                let weekday = today.weekday();
                let start_of_week = if weekday == WEEK_START {
                    today
                } else {
                    today - chrono::Duration::days(weekday.num_days_from_monday() as i64)
                };

                let end_of_week = start_of_week + chrono::Duration::days(6);

                NaiveDateTime::new(start_of_week, MIN_TIME)
                    ..NaiveDateTime::new(end_of_week, MAX_TIME)
            }
            Self::LastWeek => {
                todo!()
            }
            Self::ThisMonth => {
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap(),
                    MIN_TIME,
                )..NaiveDateTime::new(now.date(), MAX_TIME)
            }
            Self::LastMonth => {
                todo!()
            }
            Self::ThisYear => {
                NaiveDateTime::new(NaiveDate::from_ymd_opt(now.year(), 1, 1).unwrap(), MIN_TIME)
                    ..NaiveDateTime::new(now.date(), MAX_TIME)
            }
            Self::LastYear => {
                todo!()
            }
        };

        Some(ret)
    }
}

mod imp {
    use std::cell::{OnceCell, RefCell};

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/date_time_picker.ui")]
    pub struct DateTimePicker {
        #[template_child]
        pub(super) range_kind_dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub(super) pickers_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub(super) range_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) from_calendar: TemplateChild<gtk::Calendar>,
        #[template_child]
        pub(super) from_time_picker: TemplateChild<TimePicker>,
        #[template_child]
        pub(super) to_calendar: TemplateChild<gtk::Calendar>,
        #[template_child]
        pub(super) to_time_picker: TemplateChild<TimePicker>,

        pub(super) result_tx: RefCell<Option<oneshot::Sender<()>>>,
        pub(super) range_kind_dropdown_selected_item_id: OnceCell<glib::SignalHandlerId>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DateTimePicker {
        const NAME: &'static str = "UetsDateTimePicker";
        type Type = super::DateTimePicker;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("date-time-picker.cancel", None, move |obj, _, _| {
                let imp = obj.imp();

                let _ = imp.result_tx.take();
            });
            klass.install_action("date-time-picker.done", None, move |obj, _, _| {
                let imp = obj.imp();

                imp.result_tx.take().unwrap().send(()).unwrap();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DateTimePicker {
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
        }

        fn dispose(&self) {
            let _ = self.result_tx.take();
        }
    }

    impl WidgetImpl for DateTimePicker {}
    impl WindowImpl for DateTimePicker {}
    impl AdwWindowImpl for DateTimePicker {}
}

glib::wrapper! {
    pub struct DateTimePicker(ObjectSubclass<imp::DateTimePicker>)
        @extends gtk::Widget, gtk::Window, adw::Window;
}

impl DateTimePicker {
    pub async fn pick(
        parent: &impl IsA<gtk::Widget>,
    ) -> Result<Option<Range<NaiveDateTime>>, oneshot::Canceled> {
        let root = parent.root().map(|r| r.downcast::<gtk::Window>().unwrap());

        let this = glib::Object::builder::<Self>()
            .property("transient-for", root)
            .property("modal", true)
            .build();
        let imp = this.imp();

        let (result_tx, result_rx) = oneshot::channel();
        imp.result_tx.replace(Some(result_tx));

        this.present();

        result_rx.await?;

        this.close();

        match this.range_kind() {
            DateTimeRangeKind::AllTime => Ok(None),
            _ => Ok(Some(this.range())),
        }
    }

    fn range(&self) -> Range<NaiveDateTime> {
        let imp = self.imp();

        get_dt(&imp.from_calendar, &imp.from_time_picker)
            ..get_dt(&imp.to_calendar, &imp.to_time_picker)
    }

    fn set_range(&self, range: Range<NaiveDateTime>) {
        let imp = self.imp();

        let _guard = imp.from_calendar.freeze_notify();
        let _guard = imp.to_calendar.freeze_notify();

        let _guard = imp.from_time_picker.freeze_notify();
        let _guard = imp.to_time_picker.freeze_notify();

        set_dt(&imp.from_calendar, &imp.from_time_picker, range.start);
        set_dt(&imp.to_calendar, &imp.to_time_picker, range.end);
    }

    fn range_kind(&self) -> DateTimeRangeKind {
        let imp = self.imp();

        imp.range_kind_dropdown
            .selected_item()
            .map_or(DateTimeRangeKind::default(), |o| {
                let item = o.downcast::<adw::EnumListItem>().unwrap();
                DateTimeRangeKind::try_from(item.value()).unwrap()
            })
    }

    fn handle_range_changed(&self) {
        let imp = self.imp();

        if self.range_kind().range().is_some_and(|a| {
            let b = self.range();
            !is_eq_ignore_subsec(a.start, b.start) || !is_eq_ignore_subsec(a.end, b.end)
        }) {
            let selected_item_notify_id = imp.range_kind_dropdown_selected_item_id.get().unwrap();
            imp.range_kind_dropdown
                .block_signal(selected_item_notify_id);
            imp.range_kind_dropdown
                .set_selected(DateTimeRangeKind::Custom.position());
            imp.range_kind_dropdown
                .unblock_signal(selected_item_notify_id);
        }

        self.update_range_label();
    }

    fn update_range_label(&self) {
        let imp = self.imp();

        let range = self.range();
        imp.range_label.set_label(&format!(
            "<b>{}</b> to <b>{}</b>",
            glib::markup_escape_text(&range.start.format(DT_FORMAT).to_string()),
            glib::markup_escape_text(&range.end.format(DT_FORMAT).to_string()),
        ));
    }

    fn update_state(&self) {
        let imp = self.imp();

        let range_kind = self.range_kind();

        let reveal_pickers = !matches!(range_kind, DateTimeRangeKind::AllTime);
        imp.pickers_revealer.set_reveal_child(reveal_pickers);

        if let Some(range) = range_kind.range() {
            self.set_range(range);
        }
    }
}

fn set_dt(calendar: &gtk::Calendar, time_picker: &TimePicker, dt: NaiveDateTime) {
    calendar.set_year(dt.year());
    calendar.set_month(dt.month0() as i32);
    calendar.set_day(dt.day() as i32);
    time_picker.set_time(NaiveTimeBoxed(dt.time()));
}

fn get_dt(calendar: &gtk::Calendar, time_picker: &TimePicker) -> NaiveDateTime {
    let (year, month, day) = calendar.date().ymd();
    let date = NaiveDate::from_ymd_opt(year, month as u32, day as u32).unwrap();
    NaiveDateTime::new(date, time_picker.time().0)
}

fn is_eq_ignore_subsec(a: NaiveDateTime, b: NaiveDateTime) -> bool {
    a.date() == b.date()
        && a.hour() == b.hour()
        && a.minute() == b.minute()
        && a.second() == b.second()
}
