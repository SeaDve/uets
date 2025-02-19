use chrono::{DateTime, Utc};

pub fn builder(title: impl Into<String>) -> ReportTableBuilder {
    ReportTableBuilder {
        title: title.into(),
        columns: Vec::new(),
        rows: Vec::new(),
        graphs: Vec::new(),
    }
}

pub fn row_builder() -> ReportTableRowBuilder {
    ReportTableRowBuilder { cells: Vec::new() }
}

pub struct ReportTableRowBuilder {
    cells: Vec<ReportTableCell>,
}

impl ReportTableRowBuilder {
    pub fn cell(mut self, cell: impl Into<ReportTableCell>) -> Self {
        self.cells.push(cell.into());
        self
    }

    pub fn build(self) -> Vec<ReportTableCell> {
        self.cells
    }
}

pub struct ReportTable {
    pub title: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<ReportTableCell>>,
    pub graphs: Vec<(String, usize, usize)>,
}

pub struct ReportTableBuilder {
    title: String,
    columns: Vec<String>,
    rows: Vec<Vec<ReportTableCell>>,
    graphs: Vec<(String, usize, usize)>,
}

impl ReportTableBuilder {
    pub fn column(mut self, title: impl Into<String>) -> Self {
        self.columns.push(title.into());
        self
    }

    pub fn rows(mut self, row: impl IntoIterator<Item = Vec<ReportTableCell>>) -> Self {
        self.rows.extend(row);
        self
    }

    pub fn graph(
        mut self,
        title: impl Into<String>,
        dt_col_idx: usize,
        val_col_idx: usize,
    ) -> Self {
        self.graphs.push((title.into(), dt_col_idx, val_col_idx));
        self
    }

    pub fn build(self) -> ReportTable {
        ReportTable {
            title: self.title,
            columns: self.columns,
            rows: self.rows,
            graphs: self.graphs,
        }
    }
}

#[derive(Debug)]
pub enum ReportTableCell {
    DateTime(DateTime<Utc>),
    U32(u32),
    String(String),
}

impl ReportTableCell {
    pub fn as_date(&self) -> Option<DateTime<Utc>> {
        match self {
            ReportTableCell::DateTime(date) => Some(*date),
            _ => None,
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        match self {
            ReportTableCell::U32(u32) => Some(*u32),
            _ => None,
        }
    }

    pub fn is_date(&self) -> bool {
        matches!(self, ReportTableCell::DateTime(_))
    }

    pub fn is_u32(&self) -> bool {
        matches!(self, ReportTableCell::U32(_))
    }
}

impl From<DateTime<Utc>> for ReportTableCell {
    fn from(date: DateTime<Utc>) -> Self {
        ReportTableCell::DateTime(date)
    }
}

impl From<u32> for ReportTableCell {
    fn from(u32: u32) -> Self {
        ReportTableCell::U32(u32)
    }
}

impl From<String> for ReportTableCell {
    fn from(string: String) -> Self {
        ReportTableCell::String(string)
    }
}
