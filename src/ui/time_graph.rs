use anyhow::Result;
use chrono::Utc;
use gtk::{glib, prelude::*, subclass::prelude::*};
use plotters_cairo::CairoBackend;

use crate::colors::{self, ColorExt};

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/time_graph.ui")]
    pub struct Graph {
        #[template_child]
        pub(super) no_data_revealer: TemplateChild<gtk::Revealer>,

        pub(super) data: RefCell<Vec<(chrono::DateTime<Utc>, u32)>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Graph {
        const NAME: &'static str = "UetsTimeGraph";
        type Type = super::TimeGraph;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Graph {
        fn dispose(&self) {
            self.dispose_template();
        }
    }

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

            self.parent_snapshot(snapshot);
        }
    }
}

glib::wrapper! {
    pub struct TimeGraph(ObjectSubclass<imp::Graph>)
        @extends gtk::Widget;
}

impl TimeGraph {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_data(&self, data: Vec<(chrono::DateTime<Utc>, u32)>) {
        let imp = self.imp();

        imp.data.replace(data);

        self.queue_draw();
        self.update_no_data_revealer();
    }

    fn update_no_data_revealer(&self) {
        let imp = self.imp();

        imp.no_data_revealer
            .set_reveal_child(imp.data.borrow().is_empty());
    }

    fn draw_graph(&self, backend: CairoBackend<'_>) -> Result<()> {
        use plotters::prelude::*;

        let imp = self.imp();
        let data = imp.data.borrow();

        if data.is_empty() {
            return Ok(());
        }

        let root = backend.into_drawing_area();

        let x_min = *data.first().map(|(x, _)| x).unwrap();
        let x_max = *data.last().map(|(x, _)| x).unwrap();

        let diff = x_max.signed_duration_since(x_min);
        let formatter = if diff.num_weeks() > 0 {
            |dt: &chrono::DateTime<Utc>| dt.format("%m/%d").to_string()
        } else if diff.num_hours() > 0 {
            |dt: &chrono::DateTime<Utc>| dt.format("%H:%M").to_string()
        } else {
            |dt: &chrono::DateTime<Utc>| dt.format("%H:%M:%S").to_string()
        };

        let y_min = *data.iter().map(|(_, y)| y).min().unwrap();
        let y_max = *data.iter().map(|(_, y)| y).max().unwrap();

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
            data.iter().copied(),
            &colors::BLUE_2.to_plotters(),
        ))?;

        chart.draw_series(PointSeries::of_element(
            data.iter().copied(),
            3,
            &colors::BLUE_4.to_plotters(),
            &|c, s, st| EmptyElement::at(c) + Circle::new((0, 0), s, st.filled()),
        ))?;

        root.present()?;

        Ok(())
    }
}
