use anyhow::{Context, Result};
use chrono::{Local, Utc};
use image::{DynamicImage, RgbImage};
use plotters::{
    backend::{PixelFormat, RGBPixel},
    prelude::*,
};

use crate::colors::{self, ColorExt};

pub fn draw_image(
    (width, height): (u32, u32),
    data: &[(chrono::DateTime<Utc>, u32)],
) -> Result<DynamicImage> {
    let mut raw = vec![0; width as usize * height as usize * RGBPixel::PIXEL_SIZE];
    let backend = BitMapBackend::<RGBPixel>::with_buffer(&mut raw, (width, height));
    draw(backend, Some(WHITE), data)?;

    let image = DynamicImage::ImageRgb8(
        RgbImage::from_raw(width, height, raw).context("Failed to create RGB image")?,
    );

    Ok(image)
}

pub fn draw<DB>(
    backend: DB,
    fill: Option<RGBColor>,
    data: &[(chrono::DateTime<Utc>, u32)],
) -> Result<()>
where
    DB: DrawingBackend,
    <DB as DrawingBackend>::ErrorType: 'static,
{
    let root = backend.into_drawing_area();

    if let Some(fill) = fill {
        root.fill(&fill)?;
    }

    let x_min = data
        .first()
        .map(|(x, _)| *x)
        .unwrap_or(chrono::DateTime::<Utc>::MIN_UTC);
    let x_max = data
        .last()
        .map(|(x, _)| *x)
        .unwrap_or(chrono::DateTime::<Utc>::MIN_UTC);

    let diff = x_max.signed_duration_since(x_min);
    let formatter = if diff.num_weeks() > 4 {
        |dt: &chrono::DateTime<Utc>| dt.with_timezone(&Local).format("%Y/%m/%d").to_string()
    } else if diff.num_days() > 0 {
        |dt: &chrono::DateTime<Utc>| dt.with_timezone(&Local).format("%m/%d").to_string()
    } else {
        |dt: &chrono::DateTime<Utc>| dt.with_timezone(&Local).format("%H:%M:%S").to_string()
    };

    let y_min = data.iter().map(|(_, y)| *y).min().unwrap_or(0);
    let y_max = data.iter().map(|(_, y)| *y).max().unwrap_or(0);

    let mut chart = ChartBuilder::on(&root)
        .margin(20)
        .x_label_area_size(20)
        .y_label_area_size(20)
        .build_cartesian_2d(x_min..x_max, y_min..y_max)?;

    chart
        .configure_mesh()
        .disable_mesh()
        .x_label_style(("Cantarell", 10))
        .x_label_formatter(&formatter)
        .x_labels(10)
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
