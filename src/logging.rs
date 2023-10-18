/// This is partially imported from https://github.com/ac-freeman/adder-codec-rs/blob/feature-eval-log/adder-codec-rs/src/utils/logging.rs
/// Really, these should be synchronized in some way (TODO?)
use aedat::events_generated::Event;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

#[derive(Debug, Clone, Copy)]
pub struct LogFeature {
    pub x: u16,
    pub y: u16,
    pub non_max_suppression: bool,
    pub source: LogFeatureSource,
}

impl Serialize for LogFeature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let mut state = serializer.serialize_struct("LogFeature", 4)?;
        state.serialize_field("x", &self.x)?;
        state.serialize_field("y", &self.y)?;
        state.serialize_field("n", &self.non_max_suppression)?;
        state.serialize_field("s", &self.source)?;
        state.end()
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum LogFeatureSource {
    ADDER,
    OpenCV,
    DVS
}


impl LogFeature {
    pub fn from_event(event: &Event) -> Self {
        Self {
            x: event.x() as u16,
            y: event.y() as u16,
            non_max_suppression: false, // TODO ?
            source: LogFeatureSource::DVS,
        }
    }
}
