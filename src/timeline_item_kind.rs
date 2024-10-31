#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineItemKind {
    Entry,
    Exit,
}

impl TimelineItemKind {
    pub fn is_entry(&self) -> bool {
        matches!(self, Self::Entry)
    }

    pub fn is_exit(&self) -> bool {
        matches!(self, Self::Exit)
    }
}
