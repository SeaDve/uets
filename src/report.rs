use anyhow::Result;
use genpdf::{
    elements::{FrameCellDecorator, Paragraph, TableLayout, Text},
    fonts,
    style::{self, StyledString},
    Alignment, Document, Element, SimplePageDecorator,
};
use gtk::gio;

use crate::{entity::Entity, GRESOURCE_PREFIX};

const DOC_LINE_SPACING: f64 = 1.5;
const DOC_MARGINS: f64 = 10.0;

pub fn gen_entities(entities: &[Entity]) -> Result<Vec<u8>> {
    let mut doc = doc()?;
    doc.set_title("Entities");

    doc.push(p_bold("Entities"));
    doc.push(p(format!(
        "Date Generated: {}",
        chrono::Local::now().to_rfc2822()
    )));
    doc.push(p(format!("Total Entities: {}", entities.len())));

    let mut table = table(3);
    table.push_row(vec![
        p_bold_centered("ID").b(),
        p_bold_centered("StockID").b(),
        p_bold_centered("Zone").b(),
    ])?;
    for entity in entities {
        table.push_row(vec![
            t(entity.id().to_string()).b(),
            t(entity
                .stock_id()
                .map(|id| id.to_string())
                .unwrap_or_default())
            .b(),
            t(if entity.is_inside() {
                "Inside"
            } else {
                "Outside"
            })
            .b(),
        ])?;
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
