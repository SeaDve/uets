use anyhow::Result;
use chrono::Utc;
use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};
use plotters_cairo::CairoBackend;

use crate::{
    colors::{self, ColorExt},
    stock_timeline::StockTimeline,
};

mod imp {
    use std::cell::{OnceCell, RefCell};

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/stock_graph.ui")]
    pub struct StockGraph {
        #[template_child]
        pub(super) no_data_revealer: TemplateChild<gtk::Revealer>,

        pub(super) timeline: RefCell<Option<StockTimeline>>,
        pub(super) timeline_signals: OnceCell<glib::SignalGroup>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StockGraph {
        const NAME: &'static str = "UetsStockGraph";
        type Type = super::StockGraph;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for StockGraph {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            let timeline_signals = glib::SignalGroup::new::<StockTimeline>();
            timeline_signals.connect_local(
                "items-changed",
                false,
                clone!(
                    #[weak]
                    obj,
                    #[upgrade_or_panic]
                    move |_| {
                        obj.queue_draw();
                        obj.update_no_data_revealer();
                        None
                    }
                ),
            );
            self.timeline_signals.set(timeline_signals).unwrap();

            obj.update_no_data_revealer();
        }

        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for StockGraph {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let obj = self.obj();

            let width = obj.width();
            let height = obj.height();

            if width == 0 || height == 0 {
                return;
            }

            let bounds = gtk::graphene::Rect::new(0.0, 0.0, width as f32, height as f32);
            let cr = snapshot.append_cairo(&bounds);
            let backend = CairoBackend::new(&cr, (width as u32, height as u32)).unwrap();

            if let Err(err) = obj.draw_graph(backend) {
                tracing::error!("Failed to draw graph: {:?}", err);
            }

            self.parent_snapshot(snapshot);
        }
    }
}

glib::wrapper! {
    pub struct StockGraph(ObjectSubclass<imp::StockGraph>)
        @extends gtk::Widget;
}

impl StockGraph {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_timeline(&self, timeline: Option<StockTimeline>) {
        let imp = self.imp();

        imp.timeline_signals
            .get()
            .unwrap()
            .set_target(timeline.as_ref());
        imp.timeline.replace(timeline);

        self.queue_draw();
        self.update_no_data_revealer();
    }

    pub fn timeline(&self) -> Option<StockTimeline> {
        self.imp().timeline.borrow().clone()
    }

    fn update_no_data_revealer(&self) {
        let imp = self.imp();

        imp.no_data_revealer.set_reveal_child(
            self.timeline().is_none() || self.timeline().is_some_and(|t| t.is_empty()),
        );
    }

    fn draw_graph(&self, backend: CairoBackend<'_>) -> Result<()> {
        use plotters::prelude::*;

        let Some(timeline) = self.timeline() else {
            return Ok(());
        };

        if timeline.is_empty() {
            return Ok(());
        }

        let root = backend.into_drawing_area();

        let x_min = timeline.first().unwrap().dt().inner();
        let x_max = timeline.last().unwrap().dt().inner();

        let diff = x_max.signed_duration_since(x_min);
        let formatter = if diff.num_weeks() > 0 {
            |dt: &chrono::DateTime<Utc>| dt.format("%m/%d").to_string()
        } else if diff.num_hours() > 0 {
            |dt: &chrono::DateTime<Utc>| dt.format("%H:%M").to_string()
        } else {
            |dt: &chrono::DateTime<Utc>| dt.format("%H:%M:%S").to_string()
        };

        let y_min = timeline.iter().map(|item| item.n_inside()).min().unwrap();
        let y_max = timeline.iter().map(|item| item.n_inside()).max().unwrap();

        let mut chart = ChartBuilder::on(&root)
            .margin(20)
            .x_label_area_size(20)
            .y_label_area_size(20)
            .build_cartesian_2d(x_min..x_max, y_min..y_max)?;

        chart
            .configure_mesh()
            .disable_mesh()
            .x_label_style(("Cantarell", 12))
            .x_label_formatter(&formatter)
            .x_labels(8)
            .y_label_style(("Cantarell", 12))
            .y_labels(8)
            .draw()?;

        chart.draw_series(LineSeries::new(
            timeline
                .iter()
                .map(|item| (item.dt().inner(), item.n_inside())),
            &colors::BLUE_2.to_plotters(),
        ))?;

        chart.draw_series(PointSeries::of_element(
            timeline
                .iter()
                .map(|item| (item.dt().inner(), item.n_inside())),
            3,
            &colors::BLUE_4.to_plotters(),
            &|c, s, st| EmptyElement::at(c) + Circle::new((0, 0), s, st.filled()),
        ))?;

        root.present()?;

        Ok(())
    }
}
