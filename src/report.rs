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
    };

    format!("{title} ({timestamp}).{extension}")
}

pub fn builder(kind: ReportKind, title: impl Into<String>) -> ReportBuilder {
    ReportBuilder {
        kind,
        title: title.into(),
        props: Vec::new(),
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
    table: Option<ReportTable>,
}

impl ReportBuilder {
    pub fn prop(mut self, key: impl Into<String>, value: impl fmt::Display) -> Self {
        self.props.push((key.into(), value.to_string()));
        self
    }

    pub fn table(mut self, table: ReportTable) -> Self {
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

    use crate::{report::ReportBuilder, time_graph, GRESOURCE_PREFIX};

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

            for row in t.rows.iter() {
                table.push_row(
                    row.iter()
                        .map(|cell| {
                            Text::new(cell.to_string())
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
        drawing::{
            charts::{AxisId, LineChart, Values},
            spreadsheet::MarkerType,
        },
        helper::coordinate::{coordinate_from_index, CellCoordinates},
        writer, Chart, ChartType, HorizontalAlignmentValues, Style,
    };

    use crate::report::ReportBuilder;

    const WORKSHEET_NAME: &str = "Sheet1";

    pub fn build(b: ReportBuilder) -> Result<Vec<u8>> {
        let mut spreadsheet = spreadsheet::new_file_empty_worksheet();
        spreadsheet.new_sheet(WORKSHEET_NAME).unwrap();
        spreadsheet.set_active_sheet(0);

        let cur_col_idx = 1_u32;
        let mut cur_row_idx = 1_u32;

        let n_columns = b
            .table
            .as_ref()
            .map_or(0, |table| table.columns.len() as u32);

        let title_style = {
            let mut style = Style::default();
            style
                .get_alignment_mut()
                .set_horizontal(HorizontalAlignmentValues::Center);
            style.get_font_mut().set_bold(true);
            style
        };

        let worksheet = spreadsheet.get_active_sheet_mut();

        let title_coord = (cur_col_idx, cur_row_idx);
        worksheet.add_merge_cells(cell_range(title_coord, (n_columns, cur_row_idx)));
        let table_title_cell = worksheet.get_cell_mut(title_coord);
        table_title_cell.set_value_string(&b.title);
        table_title_cell.set_style(title_style.clone());
        cur_row_idx += 1;

        for (name, value) in b.props {
            let name_cell = worksheet.get_cell_mut((cur_col_idx, cur_row_idx));
            name_cell.set_value_string(&name);

            let value_cell = worksheet.get_cell_mut((cur_col_idx + 1, cur_row_idx));
            value_cell.set_value_string(&value);

            cur_row_idx += 1;
        }

        cur_row_idx += 1;

        if let Some(t) = b.table {
            let table_title_coord = (cur_col_idx, cur_row_idx);
            worksheet.add_merge_cells(cell_range(table_title_coord, (n_columns, cur_row_idx)));
            let table_title_cell = worksheet.get_cell_mut(table_title_coord);
            table_title_cell.set_value_string(t.title);
            table_title_cell.set_style(title_style.clone());
            cur_row_idx += 1;

            let table_row_start = cur_row_idx;

            for (col_idx, col_title) in t.columns.into_iter().enumerate() {
                let col_idx = col_idx as u32 + cur_col_idx;

                let cell = worksheet.get_cell_mut((col_idx, cur_row_idx));
                cell.set_style(title_style.clone());
                cell.set_value_string(col_title);
            }
            cur_row_idx += 1;

            for row in t.rows.into_iter() {
                for (col_idx, value) in row.into_iter().enumerate() {
                    let col_idx = col_idx as u32 + cur_col_idx;

                    let cell = worksheet.get_cell_mut((col_idx, cur_row_idx));
                    cell.set_value_string(value.to_string());
                }

                cur_row_idx += 1;
            }

            let table_row_end = cur_row_idx - 1;

            // TODO add graphs
            // for (graph_title, dt_col_idx, val_col_idx) in t.graphs {
            //     let mut from_marker = MarkerType::default();
            //     from_marker.set_coordinate(cell((cur_col_idx, cur_row_idx)));

            //     let mut to_marker = MarkerType::default();
            //     to_marker.set_coordinate(cell((cur_col_idx + 10, cur_row_idx + 10)));

            //     // let axis_id = AxisId::default();

            //     // let mut chart = LineChart::default();
            //     // chart.set_axis_id(value)

            //     Values::default().get_number_reference_mut();

            //     let mut chart = Chart::default();
            //     chart.new_chart(
            //         ChartType::LineChart,
            //         from_marker,
            //         to_marker,
            //         vec![&sheet_cell_range(
            //             WORKSHEET_NAME,
            //             ((val_col_idx + 1) as u32, table_row_start),
            //             ((val_col_idx + 1) as u32, table_row_end),
            //         )],
            //     );
            //     chart.set_title(graph_title);

            //     worksheet.add_chart(chart);

            //     cur_row_idx += 12;
            // }
        }

        let mut bytes = Vec::new();
        writer::xlsx::write_writer(&spreadsheet, &mut bytes)?;
        Ok(bytes)
    }

    fn sheet_cell_range(
        sheet_name: &str,
        start: impl Into<CellCoordinates>,
        end: impl Into<CellCoordinates>,
    ) -> String {
        let start = start.into();
        let end = end.into();

        format!("{}!{}", sheet_name, cell_range(start, end))
    }

    fn cell_range(start: impl Into<CellCoordinates>, end: impl Into<CellCoordinates>) -> String {
        let start_coords = start.into();
        let end_coords = end.into();

        format!(
            "{}:{}",
            coordinate_from_index(&(start_coords.col), &(start_coords.row)),
            coordinate_from_index(&(end_coords.col), &(end_coords.row))
        )
    }

    fn cell(coords: impl Into<CellCoordinates>) -> String {
        let coords = coords.into();
        coordinate_from_index(&(coords.col), &(coords.row))
    }
}
