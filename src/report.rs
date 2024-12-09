use std::{fmt, time::Instant};

use anyhow::Result;
use chrono::Local;

use gtk::{gio, glib};

use crate::report_table::ReportTable;

pub fn file_name(title: &str, kind: ReportKind) -> String {
    let timestamp = Local::now().format("%Y-%m-%d-%H-%M-%S");
    let extension = match kind {
        ReportKind::Pdf => "pdf",
        ReportKind::Spreadsheet => "xlsx",
        ReportKind::Csv => "csv",
    };

    format!("{title} ({timestamp}).{extension}")
}

pub fn builder(kind: ReportKind, title: impl Into<String>) -> ReportBuilder {
    ReportBuilder {
        kind,
        title: title.into(),
        props: vec![("Date Generated".into(), Local::now().to_rfc2822())],
        table: None,
    }
}

#[derive(Debug, Clone, Copy, glib::Variant)]
pub enum ReportKind {
    Pdf,
    Spreadsheet,
    Csv,
}

pub struct ReportBuilder {
    kind: ReportKind,
    title: String,
    props: Vec<(String, String)>,
    table: Option<ReportTable>,
}

impl ReportBuilder {
    pub fn prop(mut self, key: impl Into<String>, value: impl fmt::Display) -> Self {
        self.props.push((key.into(), value.to_string()));
        self
    }

    pub fn table(mut self, table: ReportTable) -> Self {
        debug_assert!(table
            .rows
            .iter()
            .all(|cells| cells.len() == table.columns.len()));
        debug_assert!(table
            .graphs
            .iter()
            .all(|(_, dt_col_idx, val_col_idx)| dt_col_idx != val_col_idx));
        debug_assert!(table
            .graphs
            .iter()
            .all(|(_, dt_col_idx, _)| *dt_col_idx < table.columns.len()));
        debug_assert!(table
            .graphs
            .iter()
            .all(|(_, _, val_col_idx)| *val_col_idx < table.columns.len()));

        debug_assert!(table
            .graphs
            .iter()
            .all(|(_, dt_col_idx, _)| table.rows.iter().all(|row| row[*dt_col_idx].is_date())));
        debug_assert!(table
            .graphs
            .iter()
            .all(|(_, _, val_col_idx)| table.rows.iter().all(|row| row[*val_col_idx].is_u32())));

        self.table = Some(table);
        self
    }

    pub async fn build(self) -> Result<Vec<u8>> {
        gio::spawn_blocking(move || {
            let now = Instant::now();

            let kind = self.kind;
            let ret = match kind {
                ReportKind::Pdf => pdf::build(self),
                ReportKind::Spreadsheet => spreadsheet::build(self),
                ReportKind::Csv => csv::build(self),
            };

            tracing::trace!("Built {:?} report in {:?}", kind, now.elapsed());

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
        elements::{Break, FrameCellDecorator, Image, Paragraph, TableLayout},
        fonts::{self, FontData, FontFamily},
        style::{self, StyledString},
        Alignment, Document, Element, Margins, SimplePageDecorator,
    };
    use gtk::gio;

    use crate::{
        report::ReportBuilder, report_table::ReportTableCell, time_graph, GRESOURCE_PREFIX,
    };

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

        for (key, value) in b.props.iter() {
            doc.push(p(format!("{}: {}", key, value)));
        }

        if let Some(t) = b.table {
            for (graph_title, dt_col_idx, val_col_idx) in t.graphs {
                doc.push(br());
                doc.push(p_bold(graph_title).aligned(Alignment::Center));

                let dts = t.rows.iter().map(|row| row[dt_col_idx].as_date().unwrap());
                let vals = t.rows.iter().map(|row| row[val_col_idx].as_u32().unwrap());
                let image_data =
                    time_graph::draw_image((800, 500), &dts.zip(vals).collect::<Vec<_>>())?;

                doc.push(
                    Image::from_dynamic_image(image_data)?
                        .with_alignment(Alignment::Center)
                        .with_scale((2.0, 2.0)),
                );
            }

            doc.push(br());
            doc.push(p_bold(t.title).aligned(Alignment::Center));

            let mut table = TableLayout::new(vec![1; t.columns.len()]);

            let cell_decorator = FrameCellDecorator::new(true, true, true);
            table.set_cell_decorator(cell_decorator);

            table.push_row(
                t.columns
                    .iter()
                    .map(|title| p_bold(title).aligned(Alignment::Center).boxed())
                    .collect(),
            )?;

            for row in t.rows.into_iter() {
                table.push_row(
                    row.into_iter()
                        .map(|cell| {
                            let cell_string = match cell {
                                ReportTableCell::DateTime(dt) => dt
                                    .with_timezone(&Local)
                                    .format("%Y/%m/%d %H:%M:%S")
                                    .to_string(),
                                ReportTableCell::U32(u32) => u32.to_string(),
                                ReportTableCell::String(string) => string,
                            };
                            Paragraph::new(cell_string)
                                .padded(Margins::trbl(
                                    TABLE_TOP_BOTTOM_PADDING_MM,
                                    TABLE_LEFT_RIGHT_PADDING_MM,
                                    TABLE_TOP_BOTTOM_PADDING_MM,
                                    TABLE_LEFT_RIGHT_PADDING_MM,
                                ))
                                .styled(style::Style::new().with_font_size(8))
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
    use chrono::Local;
    use xlsxwriter::{Chart, ChartType, ColNum, Format, FormatAlign, Workbook, Worksheet};

    use crate::{report::ReportBuilder, report_table::ReportTableCell};

    const WORKSHEET_NAME: &str = "Sheet1";

    const ROW_HEIGHT_PX: u16 = 20;
    const COLUMN_WIDTH_PX: u16 = 140;

    const CHART_WIDTH_COLUMNS: u32 = 10;
    const CHART_HEIGHT_ROWS: u32 = 22;

    const CHART_WIDTH_PX: u32 = COLUMN_WIDTH_PX as u32 * CHART_WIDTH_COLUMNS;
    const CHART_HEIGHT_PX: u32 = ROW_HEIGHT_PX as u32 * CHART_HEIGHT_ROWS;

    pub fn build(b: ReportBuilder) -> Result<Vec<u8>> {
        let mut book = Workbook::new();

        let sheet = {
            let mut sheet = Worksheet::new();
            sheet.set_name(WORKSHEET_NAME)?;
            sheet.set_default_row_height_pixels(ROW_HEIGHT_PX);

            book.push_worksheet(sheet);
            book.worksheet_from_index(0).unwrap()
        };

        let bold_format = Format::new().set_bold();
        let title_format = Format::new().set_align(FormatAlign::Center).set_bold();
        let dt_format = Format::new().set_num_format("yyyy/mm/dd hh:mm:ss");

        let last_col_idx = {
            let min_last_col_idx = if b.props.is_empty() { 0 } else { 1 };
            b.table.as_ref().map_or(min_last_col_idx, |table| {
                (table.columns.len() as ColNum - 1).clamp(min_last_col_idx, ColNum::MAX)
            })
        };

        for col_idx in 0..=last_col_idx {
            sheet.set_column_width_pixels(col_idx, COLUMN_WIDTH_PX)?;
        }

        let mut cur_row_idx = 0;

        sheet.merge_range(
            cur_row_idx,
            0,
            cur_row_idx,
            last_col_idx,
            &b.title,
            &title_format,
        )?;
        cur_row_idx += 1;

        for (name, val) in b.props {
            sheet.write_string_with_format(cur_row_idx, 0, &name, &bold_format)?;
            sheet.write_string(cur_row_idx, 1, &val)?;
            cur_row_idx += 1;
        }
        cur_row_idx += 1;

        if let Some(t) = b.table {
            sheet.merge_range(
                cur_row_idx,
                0,
                cur_row_idx,
                last_col_idx,
                &t.title,
                &title_format,
            )?;
            cur_row_idx += 1;

            for (col_idx, col_title) in t.columns.into_iter().enumerate() {
                sheet.write_string_with_format(
                    cur_row_idx,
                    col_idx as ColNum,
                    &col_title,
                    &title_format,
                )?;
            }
            cur_row_idx += 1;

            let table_start_row_idx = cur_row_idx;

            for row in t.rows.iter() {
                for (col_idx, cell) in row.iter().enumerate() {
                    match cell {
                        ReportTableCell::DateTime(dt) => {
                            sheet.write_datetime_with_format(
                                cur_row_idx,
                                col_idx as ColNum,
                                dt.with_timezone(&Local).naive_local(),
                                &dt_format,
                            )?;
                        }
                        ReportTableCell::U32(u32) => {
                            sheet.write_number(cur_row_idx, col_idx as ColNum, *u32 as f64)?;
                        }
                        ReportTableCell::String(string) => {
                            sheet.write_string(cur_row_idx, col_idx as ColNum, string)?;
                        }
                    }
                }
                cur_row_idx += 1;
            }

            let table_end_row_idx = if t.rows.is_empty() {
                table_start_row_idx
            } else {
                cur_row_idx - 1
            };

            cur_row_idx += 1;

            for (graph_title, dt_col_idx, val_col_idx) in t.graphs {
                let mut chart = Chart::new(ChartType::ScatterStraightWithMarkers);
                chart.set_width(CHART_WIDTH_PX).set_height(CHART_HEIGHT_PX);
                chart.title().set_name(&graph_title);
                chart.legend().set_hidden();
                chart
                    .add_series()
                    .set_values((
                        WORKSHEET_NAME,
                        table_start_row_idx,
                        val_col_idx as ColNum,
                        table_end_row_idx,
                        val_col_idx as ColNum,
                    ))
                    .set_categories((
                        WORKSHEET_NAME,
                        table_start_row_idx,
                        dt_col_idx as ColNum,
                        table_end_row_idx,
                        dt_col_idx as ColNum,
                    ));
                sheet.insert_chart(cur_row_idx, 0, &chart)?;
                cur_row_idx += CHART_HEIGHT_ROWS + 1;
            }
        }

        Ok(book.save_to_buffer()?)
    }
}

mod csv {
    use anyhow::Result;
    use chrono::Local;
    use csv::Writer;

    use crate::{report::ReportBuilder, report_table::ReportTableCell};

    pub fn build(b: ReportBuilder) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();

        if let Some(t) = b.table {
            let mut w = Writer::from_writer(&mut bytes);

            w.write_record(t.columns)?;

            for row in t.rows.into_iter() {
                w.write_record(row.into_iter().map(|r| {
                    match r {
                        ReportTableCell::DateTime(dt) => dt
                            .with_timezone(&Local)
                            .format("%Y/%m/%d %H:%M:%S")
                            .to_string(),
                        ReportTableCell::U32(u32) => u32.to_string(),
                        ReportTableCell::String(string) => string,
                    }
                }))?;
            }
        }

        Ok(bytes)
    }
}
