use std::{fmt, time::Instant};

use anyhow::Result;
use chrono::Local;

use gtk::{gio, glib};
use image::DynamicImage;

pub fn file_name(title: &str, kind: ReportKind) -> String {
    let timestamp = Local::now().format("%Y-%m-%d-%H-%M-%S");
    let extension = match kind {
        ReportKind::Pdf => "pdf",
        ReportKind::Spreadsheet => "xlsx",
    };

    format!("{title} ({timestamp}).{extension}")
}

pub fn builder(kind: ReportKind, title: impl Into<String>) -> ReportBuilder {
    ReportBuilder {
        kind,
        title: title.into(),
        props: Vec::new(),
        images: Vec::new(),
        table: None,
    }
}

#[derive(Debug, Clone, Copy, glib::Variant)]
pub enum ReportKind {
    Pdf,
    Spreadsheet,
}

pub struct ReportBuilder {
    kind: ReportKind,
    title: String,
    props: Vec<(String, String)>,
    images: Vec<(String, DynamicImage)>,
    table: Option<(String, Vec<String>, Vec<Vec<String>>)>,
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
        title: impl Into<String>,
        col_titles: impl IntoIterator<Item = impl Into<String>>,
        rows: impl IntoIterator<Item = impl IntoIterator<Item = impl Into<String>>>,
    ) -> Self {
        let col_titles = col_titles
            .into_iter()
            .map(|col_title| col_title.into())
            .collect::<Vec<_>>();
        let rows = rows
            .into_iter()
            .map(|row| row.into_iter().map(|cell| cell.into()).collect::<Vec<_>>())
            .collect::<Vec<_>>();

        debug_assert!(rows.iter().all(|row| row.len() == col_titles.len()));

        self.table = Some((title.into(), col_titles, rows));
        self
    }

    pub async fn build(self) -> Result<Vec<u8>> {
        gio::spawn_blocking(move || {
            let now = Instant::now();

            let kind = self.kind;
            let ret = match kind {
                ReportKind::Pdf => pdf::build(self),
                ReportKind::Spreadsheet => spreadsheet::build(self),
            };

            tracing::debug!("Built {:?} report in {:?}", kind, now.elapsed());

            ret
        })
        .await
        .unwrap()
    }
}

mod pdf {
    use std::sync::LazyLock;

    use anyhow::Result;
    use chrono::Local;
    use genpdf::{
        elements::{Break, FrameCellDecorator, Image, Paragraph, TableLayout, Text},
        fonts::{self, FontData, FontFamily},
        style::{self, StyledString},
        Alignment, Document, Element, Margins, SimplePageDecorator,
    };
    use gtk::gio;

    use crate::{report::ReportBuilder, GRESOURCE_PREFIX};

    const DOC_LINE_SPACING_MM: f64 = 1.5;
    const DOC_MARGINS_MM: f64 = 10.0;

    const TABLE_TOP_BOTTOM_PADDING_MM: f64 = 0.0;
    const TABLE_LEFT_RIGHT_PADDING_MM: f64 = 1.0;

    static DEFAULT_FONT_FAMILY: LazyLock<FontFamily<FontData>> =
        LazyLock::new(|| fonts::FontFamily {
            regular: font_data_from_resource("times.ttf").unwrap(),
            bold: font_data_from_resource("timesbd.ttf").unwrap(),
            italic: font_data_from_resource("timesi.ttf").unwrap(),
            bold_italic: font_data_from_resource("timesbi.ttf").unwrap(),
        });

    trait Boxed {
        fn boxed(self) -> Box<dyn Element>;
    }

    impl<T: Element + 'static> Boxed for T {
        fn boxed(self) -> Box<dyn Element> {
            Box::new(self)
        }
    }

    pub fn build(b: ReportBuilder) -> Result<Vec<u8>> {
        let mut doc = Document::new(DEFAULT_FONT_FAMILY.clone());
        doc.set_minimal_conformance();
        doc.set_line_spacing(DOC_LINE_SPACING_MM);

        let mut decorator = SimplePageDecorator::new();
        decorator.set_margins(DOC_MARGINS_MM);
        doc.set_page_decorator(decorator);

        doc.set_title(b.title.clone());
        doc.push(p_bold(b.title).styled(style::Style::new().with_font_size(24)));

        doc.push(p(format!("Date Generated: {}", Local::now().to_rfc2822())));
        for (key, value) in b.props.iter() {
            doc.push(p(format!("{}: {}", key, value)));
        }

        for (image_title, image_data) in b.images {
            doc.push(br());
            doc.push(p_bold(image_title).aligned(Alignment::Center));

            doc.push(
                Image::from_dynamic_image(image_data)?
                    .with_alignment(Alignment::Center)
                    .with_scale((2.0, 2.0)),
            );
        }

        if let Some((table_title, col_titles, rows)) = b.table {
            doc.push(br());
            doc.push(p_bold(table_title).aligned(Alignment::Center));

            let mut table = TableLayout::new(vec![1; col_titles.len()]);

            let cell_decorator = FrameCellDecorator::new(true, true, true);
            table.set_cell_decorator(cell_decorator);

            table.push_row(
                col_titles
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
        }

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

    #[must_use]
    fn p_bold(text: impl Into<String>) -> Paragraph {
        Paragraph::new(StyledString::new(text, style::Effect::Bold))
    }

    #[must_use]
    fn p(text: impl Into<String>) -> Paragraph {
        Paragraph::new(text.into())
    }

    #[must_use]
    fn br() -> Break {
        Break::new(1)
    }
}

mod spreadsheet {
    use anyhow::Result;
    use spreadsheet::{
        helper::coordinate::coordinate_from_index, writer, HorizontalAlignmentValues, Style,
    };

    use crate::report::ReportBuilder;

    pub fn build(b: ReportBuilder) -> Result<Vec<u8>> {
        let mut spreadsheet = spreadsheet::new_file();

        // TODO handle title, props, and images (graphs).

        if let Some((table_title, col_titles, rows)) = b.table {
            let worksheet = spreadsheet.get_active_sheet_mut();

            let col_start = 1;
            let mut row_start = 1;

            let title_style = {
                let mut style = Style::default();
                style
                    .get_alignment_mut()
                    .set_horizontal(HorizontalAlignmentValues::Center);
                style.get_font_mut().set_bold(true);
                style
            };

            let title_start_coord = coordinate_from_index(&(col_start as u32), &(row_start as u32));
            let title_end_coord =
                coordinate_from_index(&(col_titles.len() as u32), &(row_start as u32));
            worksheet.add_merge_cells(format!("{title_start_coord}:{title_end_coord}"));
            let title_cell = worksheet.get_cell_mut(title_start_coord);
            title_cell.set_value_string(table_title);
            title_cell.set_style(title_style.clone());
            row_start += 1;

            for (col_idx, col_title) in col_titles.into_iter().enumerate() {
                let col_idx = col_idx + col_start;

                let cell = worksheet.get_cell_mut((col_idx as u32, row_start as u32));
                cell.set_style(title_style.clone());
                cell.set_value_string(col_title);
            }
            row_start += 1;

            for (row_idx, row) in rows.into_iter().enumerate() {
                let row_idx = row_idx + row_start;

                for (col_idx, value) in row.into_iter().enumerate() {
                    let col_idx = col_idx + col_start;

                    let cell = worksheet.get_cell_mut((col_idx as u32, row_idx as u32));
                    cell.set_value_string(value);
                }
            }
        }

        let mut bytes = Vec::new();
        writer::xlsx::write_writer(&spreadsheet, &mut bytes)?;
        Ok(bytes)
    }
}
