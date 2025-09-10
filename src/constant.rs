use std::fmt;

use crate::nistem::Color;


pub const DEFAULT_MODEL: &str = "https://github.com/mixxxdj/demucs/releases/latest/download/htdemucs.onnx";
pub const STEM_DEFAULT_LABEL: [&str; 4] = [
    "Drums",
    "Bass",
    "Other",
    "Vocals"];
pub const STEM_DEFAULT_COLOR: [Color; 4] = [
    Color(0x009E73),
    Color(0xD55E00),
    Color(0xCC79A7),
    Color(0x56B4E9),
];

#[derive(Debug, Clone, PartialEq, Eq, Copy, Hash)]
pub enum Metadata {
    Title,
    Artist,
    Release,
    Label,
    Genre,
    TrackNo,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MetadataValue {
    String(String),
    Number(u32),
}

impl From<String> for MetadataValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<u32> for MetadataValue {
    fn from(value: u32) -> Self {
        Self::Number(value)
    }
}

impl ToString for MetadataValue {
    fn to_string(&self) -> String {
        match self {
            MetadataValue::String(value) => value.to_owned(),
            MetadataValue::Number(value) => value.to_string(),
        }
    }
}

impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
       match self {
           Metadata::Title => write!(f, "Title"),
           Metadata::Artist => write!(f, "Artist"),
           Metadata::Release => write!(f, "Release"),
           Metadata::Label => write!(f, "Label"),
           Metadata::Genre => write!(f, "Genre"),
           Metadata::TrackNo => write!(f, "Track No"),
       }
    }
}
