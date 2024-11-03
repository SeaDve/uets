use std::fmt;

use anyhow::Result;
use chrono::Local;
use genpdf::{
    elements::{Break, FrameCellDecorator, Image, Paragraph, TableLayout, Text},
    fonts,
    style::{self, StyledString},
    Alignment, Document, Element, Margins, SimplePageDecorator,
};
use gtk::gio;
use image::DynamicImage;

use crate::GRESOURCE_PREFIX;

const DOC_LINE_SPACING_MM: f64 = 1.5;
const DOC_MARGINS_MM: f64 = 10.0;

const TABLE_TOP_BOTTOM_PADDING_MM: f64 = 0.0;
const TABLE_LEFT_RIGHT_PADDING_MM: f64 = 1.0;

pub fn file_name(title: &str) -> String {
    format!(
        "{} ({}).pdf",
        title,
        Local::now().format("%Y-%m-%d-%H-%M-%S")
    )
}

pub fn builder(title: impl Into<String>) -> ReportBuilder {
    ReportBuilder {
        title: title.into(),
        props: Vec::new(),
        images: Vec::new(),
        rows_titles: Vec::new(),
        rows: Vec::new(),
    }
}

pub struct ReportBuilder {
    title: String,
    props: Vec<(String, String)>,
    images: Vec<(String, DynamicImage)>,
    rows_titles: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl ReportBuilder {
    pub fn prop(mut self, key: impl Into<String>, value: impl fmt::Display) -> Self {
        self.props.push((key.into(), value.to_string()));
        self
    }

    pub fn image(mut self, title: impl Into<String>, image: DynamicImage) -> Self {
        self.images.push((title.into(), image));
        self
    }

    pub fn table(
        mut self,
        row_titles: impl IntoIterator<Item = impl Into<String>>,
        rows: impl IntoIterator<Item = impl IntoIterator<Item = impl Into<String>>>,
    ) -> Self {
        self.rows_titles = row_titles
            .into_iter()
            .map(|row_title| row_title.into())
            .collect();
        self.rows = rows
            .into_iter()
            .map(|row| row.into_iter().map(|cell| cell.into()).collect())
            .collect();
        self
    }

    pub async fn build(self) -> Result<Vec<u8>> {
        gio::spawn_blocking(move || {
            gen_bytes(
                self.title,
                self.props,
                self.images,
                self.rows_titles,
                self.rows,
            )
        })
        .await
        .unwrap()
    }
}

fn gen_bytes(
    title: String,
    props: Vec<(String, String)>,
    images: Vec<(String, DynamicImage)>,
    rows_titles: Vec<String>,
    rows: Vec<Vec<String>>,
) -> Result<Vec<u8>> {
    debug_assert!(!rows.is_empty());
    debug_assert!(rows.iter().all(|row| row.len() == rows_titles.len()));

    let mut doc = doc()?;
    doc.set_title(title.clone());
    doc.set_minimal_conformance();

    doc.push(p_bold(title).styled(style::Style::new().with_font_size(24)));

    doc.push(p(format!("Date Generated: {}", Local::now().to_rfc2822())));
    for (key, value) in props.iter() {
        doc.push(p(format!("{}: {}", key, value)));
    }

    for (image_title, image_data) in images {
        doc.push(b());

        doc.push(p_bold(image_title).aligned(Alignment::Center));

        doc.push(
            Image::from_dynamic_image(image_data)?
                .with_alignment(Alignment::Center)
                .with_scale((2.0, 2.0)),
        );
    }

    doc.push(b());

    let mut table = table(rows_titles.len());
    table.push_row(
        rows_titles
            .iter()
            .map(|title| p_bold(title).aligned(Alignment::Center).boxed())
            .collect(),
    )?;
    for row in rows.iter() {
        table.push_row(
            row.iter()
                .map(|cell| {
                    Text::new(cell)
                        .padded(Margins::trbl(
                            TABLE_TOP_BOTTOM_PADDING_MM,
                            TABLE_LEFT_RIGHT_PADDING_MM,
                            TABLE_TOP_BOTTOM_PADDING_MM,
                            TABLE_LEFT_RIGHT_PADDING_MM,
                        ))
                        .boxed()
                })
                .collect(),
        )?;
    }
    doc.push(table);

    let mut bytes = Vec::new();
    doc.render(&mut bytes)?;
    Ok(bytes)
}

fn font_data_from_resource(file_name: &str) -> Result<fonts::FontData> {
    let bytes = gio::resources_lookup_data(
        &format!("{}fonts/{}", GRESOURCE_PREFIX, file_name),
        gio::ResourceLookupFlags::NONE,
    )?;
    let data = fonts::FontData::new(bytes.to_vec(), None)?;
    Ok(data)
}

fn doc() -> Result<Document> {
    let font_family = fonts::FontFamily {
        regular: font_data_from_resource("times.ttf")?,
        bold: font_data_from_resource("timesbd.ttf")?,
        italic: font_data_from_resource("timesi.ttf")?,
        bold_italic: font_data_from_resource("timesbi.ttf")?,
    };

    let mut doc = Document::new(font_family);
    doc.set_line_spacing(DOC_LINE_SPACING_MM);

    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(DOC_MARGINS_MM);
    doc.set_page_decorator(decorator);

    Ok(doc)
}

#[must_use]
fn table(n_columns: usize) -> TableLayout {
    let mut table = TableLayout::new(vec![1; n_columns]);

    let cell_decorator = FrameCellDecorator::new(true, true, true);
    table.set_cell_decorator(cell_decorator);

    table
}

#[must_use]
fn p_bold(text: impl Into<String>) -> Paragraph {
    Paragraph::new(StyledString::new(text, style::Effect::Bold))
}

#[must_use]
fn p(text: impl Into<String>) -> Paragraph {
    Paragraph::new(text.into())
}

#[must_use]
fn b() -> Break {
    Break::new(1)
}

trait Boxed {
    fn boxed(self) -> Box<dyn Element>;
}

impl<T: Element + 'static> Boxed for T {
    fn boxed(self) -> Box<dyn Element> {
        Box::new(self)
    }
}
