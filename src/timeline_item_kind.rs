use chrono::TimeDelta;

use crate::db;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineItemKind {
    Entry,
    Exit { inside_duration: TimeDelta },
}

impl TimelineItemKind {
    pub fn from_db(raw: db::RawTimelineItemKind) -> Self {
        match raw {
            db::RawTimelineItemKind::Entry => Self::Entry,
            db::RawTimelineItemKind::Exit {
                inside_duration: raw_inside_duration,
            } => Self::Exit {
                inside_duration: TimeDelta::from_std(raw_inside_duration).unwrap(),
            },
        }
    }

    pub fn to_db(self) -> db::RawTimelineItemKind {
        match self {
            Self::Entry => db::RawTimelineItemKind::Entry,
            Self::Exit { inside_duration } => db::RawTimelineItemKind::Exit {
                inside_duration: inside_duration.to_std().unwrap(),
            },
        }
    }
}
