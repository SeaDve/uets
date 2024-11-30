use anyhow::Result;
use gst::prelude::*;

use crate::GRESOURCE_PREFIX;

#[derive(Debug, Clone, Copy)]
pub enum Sound {
    DetectedSuccess,
    DetectedError,
}

impl Sound {
    pub fn play(&self) {
        if let Err(err) = self.play_inner() {
            tracing::error!(?self, "Failed to play sound: {:?}", err);
        }
    }

    fn play_inner(&self) -> Result<()> {
        let uri = format!("resource://{}sounds/{}", GRESOURCE_PREFIX, self.file_name());
        let playbin = gst::ElementFactory::make("playbin")
            .property("uri", uri)
            .build()?;
        playbin.set_state(gst::State::Playing)?;

        Ok(())
    }

    fn file_name(&self) -> &str {
        match self {
            Sound::DetectedSuccess => "detected-success.mp3",
            Sound::DetectedError => "detected-error.mp3",
        }
    }
}
