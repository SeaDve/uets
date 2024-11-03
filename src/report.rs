use anyhow::Result;
use chrono::Local;
use genpdf::{
    elements::{FrameCellDecorator, Paragraph, TableLayout, Text},
    fonts,
    style::{self, StyledString},
    Alignment, Document, Element, SimplePageDecorator,
};
use gtk::gio;

use crate::GRESOURCE_PREFIX;

const DOC_LINE_SPACING: f64 = 1.5;
const DOC_MARGINS: f64 = 10.0;

pub fn file_name(title: &str) -> String {
    format!(
        "{} ({}).pdf",
        title,
        Local::now().format("%Y-%m-%d-%H-%M-%S")
    )
}

pub async fn gen(
    title: impl Into<String>,
    props: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    rows_titles: impl IntoIterator<Item = impl Into<String>>,
    rows: impl IntoIterator<Item = impl IntoIterator<Item = impl Into<String>>>,
) -> Result<Vec<u8>> {
    let title = title.into();
    let props = props
        .into_iter()
        .map(|(key, value)| (key.into(), value.into()))
        .collect();
    let rows_titles = rows_titles
        .into_iter()
        .map(|title| title.into())
        .collect::<Vec<_>>();
    let rows = rows
        .into_iter()
        .map(|row| row.into_iter().map(|cell| cell.into()).collect::<Vec<_>>())
        .collect::<Vec<_>>();

    gio::spawn_blocking(move || gen_inner(title, props, rows_titles, rows))
        .await
        .unwrap()
}

fn gen_inner(
    title: String,
    props: Vec<(String, String)>,
    rows_titles: Vec<String>,
    rows: Vec<Vec<String>>,
) -> Result<Vec<u8>> {
    debug_assert!(!rows.is_empty());
    debug_assert!(rows.iter().all(|row| row.len() == rows_titles.len()));

    let mut doc = doc()?;
    doc.set_title(title.clone());
    doc.set_minimal_conformance();

    doc.push(p_bold(title));

    doc.push(p(format!("Date Generated: {}", Local::now().to_rfc2822())));
    for (key, value) in props.iter() {
        doc.push(p(format!("{}: {}", key, value)));
    }

    let mut table = table(rows_titles.len());

    table.push_row(
        rows_titles
            .iter()
            .map(|title| p_bold_centered(title).b())
            .collect(),
    )?;

    for row in rows.iter() {
        table.push_row(row.iter().map(|cell| t(cell).b()).collect())?;
    }

    doc.push(table);

    doc_to_bytes(doc)
}

fn doc() -> Result<Document> {
    let font_family = fonts::FontFamily {
        regular: font_data_from_resource("times.ttf")?,
        bold: font_data_from_resource("timesbd.ttf")?,
        italic: font_data_from_resource("timesi.ttf")?,
        bold_italic: font_data_from_resource("timesbi.ttf")?,
    };

    let mut doc = Document::new(font_family);
    doc.set_line_spacing(DOC_LINE_SPACING);

    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(DOC_MARGINS);
    doc.set_page_decorator(decorator);

    Ok(doc)
}

fn table(n_columns: usize) -> TableLayout {
    let mut table = TableLayout::new(vec![1; n_columns]);

    let cell_decorator = FrameCellDecorator::new(true, true, true);
    table.set_cell_decorator(cell_decorator);

    table
}

fn p_bold_centered(text: impl Into<String>) -> Paragraph {
    p_bold(text).aligned(Alignment::Center)
}

fn p_bold(text: impl Into<String>) -> Paragraph {
    Paragraph::new(StyledString::new(text, style::Effect::Bold))
}

fn p(text: impl Into<String>) -> Paragraph {
    Paragraph::new(text.into())
}

fn t(text: impl Into<String>) -> Text {
    Text::new(text.into())
}

fn font_data_from_resource(file_name: &str) -> Result<fonts::FontData> {
    let bytes = gio::resources_lookup_data(
        &format!("{}fonts/{}", GRESOURCE_PREFIX, file_name),
        gio::ResourceLookupFlags::NONE,
    )?;
    let data = fonts::FontData::new(bytes.to_vec(), None)?;
    Ok(data)
}

fn doc_to_bytes(doc: Document) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    doc.render(&mut bytes)?;
    Ok(bytes)
}

trait Boxed {
    fn b(self) -> Box<dyn Element>;
}

impl<T: Element + 'static> Boxed for T {
    fn b(self) -> Box<dyn Element> {
        Box::new(self)
    }
}
