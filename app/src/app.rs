use std::{path::PathBuf, sync::Arc};

use iced::{futures::channel::mpsc, Color, Element, Event};
use stemgen::constant::{Metadata, MetadataValue};
use uuid::Uuid;

use crate::{
    modal::{color_picker, file_conflict, settings},
    model::{RenderedFile, TrackLabel},
    File,
};

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq)]
pub enum Stage {
    #[default]
    FileSelection,
    Splitting,
    StemEdition,
    Exporting,
    Finished,
}

#[derive(Debug, Clone)]
pub enum Server {
    Ready(mpsc::Sender<Server>),
    LoadFiles(Vec<PathBuf>),
    SplitFile(Uuid, PathBuf),
}

#[derive(Debug, Clone)]
pub enum Player {
    Ready(mpsc::Sender<Player>),
    PlayFile(Uuid, u8),
    Progress(f32),
    Seek(f32),
    Stop,
}

#[derive(Debug, Clone)]
pub enum Modal {
    FileConflict,
    Settings,
    ColorPicker(Color),
}

impl Modal {
    pub fn build<'a>(&self) -> Element<'a, Message> {
        match self {
            Modal::FileConflict => file_conflict::new(),
            Modal::Settings => settings::new(),
            Modal::ColorPicker(color) => color_picker::new(color),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Toggle(Uuid),
    Remove(Uuid),
    EventOccurred(Event),
    Server(Server),
    Player(Player),
    AddNewFile,
    Processing,
    Split,
    Export,
    Quit,
    Modal(Modal),
    CloseModal,
    Cancel,
    // FileUpdated(Uuid),
    FileSplitProgress(Uuid, f32),
    FileSplitCompleted(Uuid, RenderedFile, Vec<Vec<(f32, f32)>>),
    Edit(Uuid),
    Preview(Uuid, bool),
    PreviewSeek(Uuid, f32),
    StemTrackRequestColorChange(Uuid, usize),
    StemTrackToggleMute(Uuid, usize),
    StemTrackColorChanged(Uuid, usize, Option<Color>),
    StemTrackLabelUpdated(Uuid, usize, TrackLabel, String),
    MetadataEdit(Uuid, Metadata, MetadataValue),
    SettingSelectModel(String),
    ColorSelected(Color),
    SettingAddCustomColor,
    SettingRemoveCustomColor(Color),
}
