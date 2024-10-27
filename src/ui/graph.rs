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
    timeline::Timeline,
};

mod imp {
    use std::cell::OnceCell;

    use super::*;

    #[derive(Default)]
    pub struct Graph {
        pub(super) timeline: OnceCell<Timeline>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Graph {
        const NAME: &'static str = "UetsGraph";
        type Type = super::Graph;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for Graph {}

    impl WidgetImpl for Graph {
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
        }
    }
}

glib::wrapper! {
    pub struct Graph(ObjectSubclass<imp::Graph>)
        @extends gtk::Widget;
}

impl Graph {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn bind_timeline(&self, timeline: &Timeline) {
        let imp = self.imp();

        timeline.connect_items_changed(clone!(
            #[weak(rename_to = obj)]
            self,
            move |_, _, _, _| {
                obj.queue_draw();
            }
        ));

        imp.timeline.set(timeline.clone()).unwrap();
    }

    fn draw_graph(&self, backend: CairoBackend<'_>) -> Result<()> {
        use plotters::prelude::*;

        let imp = self.imp();
        let timeline = imp.timeline.get().unwrap();

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
