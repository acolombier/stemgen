use crate::nistem::Color;


pub const DEFAULT_MODEL: &str = "https://github.com/acolombier/demucs/releases/download/v4.0.1-18-g1640988-onnxmodel/htdemucs.onnx";
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

impl std::fmt::Display for MetadataValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetadataValue::String(s) => f.write_str(s),
            MetadataValue::Number(i) => write!(f, "{}", i),
        }
    }
}
 impl ToString for Metadata {
    fn to_string(&self) -> String {
        match self {
            Metadata::Title => "Title".to_owned(),
            Metadata::Artist => "Artist".to_owned(),
            Metadata::Release => "Release".to_owned(),
            Metadata::Label => "Label".to_owned(),
            Metadata::Genre => "Genre".to_owned(),
            Metadata::TrackNo => "Track No".to_owned(),
        }
    }
 }
