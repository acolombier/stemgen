use std::{
    collections::HashMap, fmt::{self, Debug}, fs::OpenOptions, hash::Hash, mem, ops::{Deref, DerefMut, Index, IndexMut, Range}, path::PathBuf, slice::SliceIndex, str::FromStr, sync::Arc
};

use ::stemgen::{
    constant::{Metadata, MetadataValue},
    track::Track,
};
use bytes::{Buf, BufMut};
use ffmpeg_next::{ffi::AVPacket, packet::Ref};
use iced::{
    Color,
    advanced::image::{self, Bytes},
    color,
};
use memmap::{MmapMut, MmapOptions};
use prost;
use prost::{Enumeration, Message};
use uuid::Uuid;

use crate::{model, waveform::SAMPLE_COUNT};

#[derive(Clone, PartialEq, Message)]
pub struct Packet {
    #[prost(bytes = "vec", tag = "1")]
    pub data: Vec<u8>,
}
#[derive(Clone, PartialEq, Message)]
pub struct Stream {
    #[prost(float, repeated, tag = "1")]
    pub samples: Vec<f32>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Chunk {
    #[prost(message, repeated, tag = "1")]
    pub packets: Vec<Packet>,
    #[prost(message, repeated, tag = "2")]
    pub streams: Vec<Stream>,
    #[prost(uint64, tag = "3")]
    pub samples_count: u64,
}

impl Packet {
    pub fn new(packet: &ffmpeg_next::Packet) -> Self {
        let mut data = vec![0u8; mem::size_of::<AVPacket>() + packet.size()];
        let ptr = packet.as_ptr();
        unsafe {
            std::ptr::copy(ptr, data.as_mut_ptr() as *mut AVPacket, 1);
        }
        if let Some(d) = packet.data() {
            data[mem::size_of::<AVPacket>()..].copy_from_slice(d);
        }
        Packet { data }
    }
}

#[derive(Debug)]
pub struct RenderedFile {
    id: Uuid,
    file: std::fs::File,
    mmap: MmapMut,
    entries: Vec<usize>,
    offset: usize,
    owned: bool, // TODO use refcount?
    total_samples: Option<u64>,
    written_samples: u64,
    partial_chunk: Option<(Chunk, usize)>,
}

impl Clone for RenderedFile {
    fn clone(&self) -> Self {
        let id = self.id.clone();
        let file = std::fs::File::create_new(
            std::env::temp_dir().join(format!("stemgen_{}", id.to_string())),
        )
        .unwrap();
        let mmap = unsafe { MmapOptions::new().map(&file).unwrap().make_mut().unwrap() };
        Self {
            id,
            file,
            mmap,
            entries: self.entries.clone(),
            offset: self.offset.clone(),
            owned: self.owned,
            total_samples: self.total_samples.clone(),
            written_samples: 0,
            partial_chunk: self.partial_chunk.clone(),
        }
    }
}

impl Buf for RenderedFile {
    fn remaining(&self) -> usize {
        if let Some(_) = self.total_samples {
            self.mmap.len() - size_of::<u64>() - self.offset
        } else {
            0
        }
    }

    fn chunk(&self) -> &[u8] {
        &self.mmap[self.offset..]
    }

    fn advance(&mut self, cnt: usize) {
        self.offset += cnt
    }
}

unsafe impl BufMut for RenderedFile {
    fn remaining_mut(&self) -> usize {
        self.mmap.len() - self.offset
    }

    unsafe fn advance_mut(&mut self, cnt: usize) {
        self.offset += cnt;
    }

    fn chunk_mut(&mut self) -> &mut bytes::buf::UninitSlice {
        bytes::buf::UninitSlice::new(&mut self.mmap[self.offset..])
    }
}

impl RenderedFile {
    pub fn new(id: Uuid) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::create_new(
            std::env::temp_dir().join(format!("stemgen_{}", id.to_string())),
        )?;
        file.set_len(1024)?;
        let mmap = unsafe { MmapOptions::new().map(&file).unwrap().make_mut().unwrap() };

        Ok(Self {
            file,
            id,
            mmap,
            owned: true,
            entries: Default::default(),
            offset: Default::default(),
            total_samples: Default::default(),
            written_samples: Default::default(),
            partial_chunk: None,
        })
    }

    pub fn existing(id: Uuid) -> Result<Self, Box<dyn std::error::Error>> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(false)
            .open(std::env::temp_dir().join(format!("stemgen_{}", id.to_string())))?;
        let mmap = unsafe { MmapOptions::new().map(&file).unwrap().make_mut().unwrap() };

        let len = mmap.len() - size_of::<u64>();
        let mut b = &mmap[len..];
        let total_samples = Some(b.get_u64());

        Ok(Self {
            file,
            id,
            mmap,
            entries: Default::default(),
            offset: Default::default(),
            owned: false,
            total_samples,
            written_samples: Default::default(),
            partial_chunk: None,
        })
    }

    pub fn write(
        &mut self,
        packets: Vec<ffmpeg_next::Packet>,
        buffers: Vec<Vec<f32>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.total_samples.is_some() {
            return Err("cannot write a completed file".into());
        }
        if buffers.len() != 5 {
            return Err("expected exactly 5 buffers".into());
        }
        let samples_count = buffers.first().unwrap().len();
        if !buffers[1..].iter().all(|s| s.len() == samples_count) {
            return Err("all buffer must have the same size".into());
        }
        self.entries.push(self.offset);

        let chunk = Chunk {
            streams: buffers
                .iter()
                .map(|buffer| Stream {
                    samples: buffer.clone(),
                })
                .collect(),
            packets: packets.iter().map(|packet| Packet::new(packet)).collect(),
            samples_count: samples_count as u64,
        };

        let len = chunk.encoded_len();
        let required = len + prost::encoding::varint::encoded_len_varint(len as u64);
        if required > self.remaining_mut() {
            self.mmap.flush()?;
            self.file.set_len((self.mmap.len() + required) as u64)?;
            self.mmap = unsafe {
                MmapOptions::new()
                    .map(&self.file)
                    .unwrap()
                    .make_mut()
                    .unwrap()
            };
        }
        self.written_samples += samples_count as u64;
        chunk.encode_length_delimited(self).map_err(|e| e.into())
    }

    pub fn complete(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.total_samples = Some(self.written_samples);
        self.put_u64(self.written_samples as u64);
        self.mmap.flush()?;
        self.file.set_len(self.offset as u64)?;
        self.mmap = unsafe { MmapOptions::new().map(&self.file)?.make_mut()? };
        self.offset = 0;
        println!("entries: {:?}", self.entries);
        Ok(())
    }

    // pub fn len(&self) -> Option<usize> {
    //     self.completed
    // }

    pub fn read<B>(
        &mut self,
        main: Option<B>,
        drum: Option<B>,
        bass: Option<B>,
        other: Option<B>,
        vocals: Option<B>,
    ) -> Result<usize, Box<dyn std::error::Error>>
    where
        B: AsRef<[f32]> + AsMut<[f32]> + Deref<Target = [f32]> + DerefMut<Target = [f32]>,
    {
        if self.total_samples.is_none() {
            return Err("cannot read while being written".into());
        }
        let mut buffers = vec![main, drum, bass, other, vocals];
        let len = buffers.iter().find_map(|b| b.as_ref().map(|b| b.len()));
        if len.is_none() {
            return Err("At least one buffer need to be passed".into());
        }
        let mut len = len.unwrap();

        if !buffers
            .iter()
            .all(|b| b.as_ref().map(|b| b.as_ref().len() == len).unwrap_or(true))
        {
            return Err("all buffer must have the same size".into());
        }
        let mut buffer_offset = 0;
        if let Some((chunk, offset)) = &mut self.partial_chunk {
            let samples_count = chunk.samples_count as usize - *offset;
            for (stream, buffer) in chunk.streams.iter().zip(buffers.iter_mut()) {
                let offset = *offset;
                match buffer.as_deref_mut() {
                    Some(buffer) => {
                        if len >= samples_count {
                            buffer[buffer_offset..buffer_offset + samples_count]
                                .copy_from_slice(&stream.samples[offset..]);
                        } else {
                            buffer[buffer_offset..buffer_offset + len]
                                .copy_from_slice(&stream.samples[offset..offset + len]);
                        }
                    }
                    _ => {}
                }
            }
            if len > samples_count {
                buffer_offset += samples_count;
                len -= samples_count;
                self.partial_chunk = None;
            } else if len < samples_count {
                buffer_offset += len;
                *offset += len;
                return Ok(buffer_offset);
            }
        }
        while self.mmap.len() - size_of::<u64>() > self.offset {
            let chunk = Chunk::decode_length_delimited(&mut *self)?;
            assert_eq!(chunk.streams.len(), 5);
            let samples_count = chunk.samples_count as usize;
            for (stream, buffer) in chunk.streams.iter().zip(buffers.iter_mut()) {
                match buffer.as_deref_mut() {
                    Some(buffer) => {
                        if len >= samples_count {
                            buffer[buffer_offset..buffer_offset + samples_count]
                                .copy_from_slice(&stream.samples);
                        } else {
                            buffer[buffer_offset..buffer_offset + len]
                                .copy_from_slice(&stream.samples[..len]);
                        }
                    }
                    _ => {}
                }
            }
            if len > samples_count {
                buffer_offset += samples_count;
                len -= samples_count;
                continue;
            } else if len < samples_count {
                buffer_offset += len;
                self.partial_chunk = Some((chunk, len));
            }
            break;
        }
        Ok(buffer_offset)
    }

    pub fn total_samples(&self) -> Option<u64> {
        self.total_samples
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn seek(&self, current: f32) -> usize {
        todo!()
    }
}

impl Drop for RenderedFile {
    fn drop(&mut self) {
        if !self.owned {
            return;
        }
        // std::fs::remove_file(std::env::temp_dir().join(format!("stemgen_{}", self.id.to_string())))
        //     .unwrap();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Copy)]
pub enum TrackLabel {
    #[default]
    Acid,
    Atmos,
    Bass,
    Bassline,
    Chords,
    Clap,
    Comp,
    Donk,
    Drone,
    Drums,
    FX,
    Guitar,
    HiHat,
    Hits,
    Hook,
    Kick,
    Lead,
    Loop,
    Melody,
    Noise,
    Pads,
    Reece,
    SFX,
    Snare,
    Stabs,
    SubBass,
    Synths,
    Toms,
    Tops,
    Vocals,
    Voices,
    Custom,
}

impl TrackLabel {
    pub const ALL: [TrackLabel; 32] = [
        TrackLabel::Acid,
        TrackLabel::Atmos,
        TrackLabel::Bass,
        TrackLabel::Bassline,
        TrackLabel::Chords,
        TrackLabel::Clap,
        TrackLabel::Comp,
        TrackLabel::Donk,
        TrackLabel::Drone,
        TrackLabel::Drums,
        TrackLabel::FX,
        TrackLabel::Guitar,
        TrackLabel::HiHat,
        TrackLabel::Hits,
        TrackLabel::Hook,
        TrackLabel::Kick,
        TrackLabel::Lead,
        TrackLabel::Loop,
        TrackLabel::Melody,
        TrackLabel::Noise,
        TrackLabel::Pads,
        TrackLabel::Reece,
        TrackLabel::SFX,
        TrackLabel::Snare,
        TrackLabel::Stabs,
        TrackLabel::SubBass,
        TrackLabel::Synths,
        TrackLabel::Toms,
        TrackLabel::Tops,
        TrackLabel::Vocals,
        TrackLabel::Voices,
        TrackLabel::Custom,
    ];
}

impl std::fmt::Display for TrackLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TrackLabel::Acid => "Acid",
                TrackLabel::Atmos => "Atmos",
                TrackLabel::Bass => "Bass",
                TrackLabel::Bassline => "Bassline",
                TrackLabel::Chords => "Chords",
                TrackLabel::Clap => "Clap",
                TrackLabel::Comp => "Comp",
                TrackLabel::Donk => "Donk",
                TrackLabel::Drone => "Drone",
                TrackLabel::Drums => "Drums",
                TrackLabel::FX => "FX",
                TrackLabel::Guitar => "Guitar",
                TrackLabel::HiHat => "HiHat",
                TrackLabel::Hits => "Hits",
                TrackLabel::Hook => "Hook",
                TrackLabel::Kick => "Kick",
                TrackLabel::Lead => "Lead",
                TrackLabel::Loop => "Loop",
                TrackLabel::Melody => "Melody",
                TrackLabel::Noise => "Noise",
                TrackLabel::Pads => "Pads",
                TrackLabel::Reece => "Reece",
                TrackLabel::SFX => "SFX",
                TrackLabel::Snare => "Snare",
                TrackLabel::Stabs => "Stabs",
                TrackLabel::SubBass => "SubBass",
                TrackLabel::Synths => "Synths",
                TrackLabel::Toms => "Toms",
                TrackLabel::Tops => "Tops",
                TrackLabel::Vocals => "Vocals",
                TrackLabel::Voices => "Voices",
                TrackLabel::Custom => "Custom",
            }
        )
    }
}

#[derive(Debug)]
pub struct Stem {
    pub color: Color,
    pub label: TrackLabel,
    pub label_text: String,
    pub muted: bool,
    pub selecting_color: bool,
    pub waveform: Option<Vec<(f32, f32)>>,
    pub current_index: usize,
}

impl Default for Stem {
    fn default() -> Self {
        Self {
            color: Color::default(),
            label: TrackLabel::default(),
            label_text: String::default(),
            muted: false,
            selecting_color: false,
            waveform: None,
            current_index: 0,
        }
    }
}

#[derive(Default, Debug)]
pub struct File {
    id: Uuid,
    pub metadata: HashMap<Metadata, MetadataValue>,
    pub path: PathBuf,
    pub progress: Option<f32>,
    pub rendered: Option<RenderedFile>,
    pub stems: Vec<Stem>,
    pub editing: bool,
    pub selected: bool,
    pub cover: Option<image::Handle>,
}

impl File {
    pub fn test() -> (Uuid, Self) {
        let id = Uuid::from_str("3c9ec5fc-2294-4b5b-a83d-970dfaadda32").unwrap();
        let generate_waveform = ||{
                            let mut buf = vec![(0.0f32, 0.0f32); SAMPLE_COUNT];
                            for (i, s) in buf.iter_mut().enumerate() {
                                s.0 = f32::sin(i as f32 / 4.0);
                                s.1 = f32::sin(i as f32 / 4.0);
                            }
                            Some(buf)
                        };
        (
            id,
            File {
                path: "/home/antoine/Music/Up To No Good - OverDrive Kick Edit.mp3".into(),
                // progress: Some(1.0),
                rendered: Some(RenderedFile::existing(id.clone()).unwrap()),
                id,
                stems: vec![
                    model::Stem {
                        color: color!(0xF40162, 1.),
                        label: model::TrackLabel::Drums,
                        waveform: generate_waveform(),
                        ..model::Stem::default()
                    },
                    model::Stem {
                        color: color!(0xFF9D0A, 1.),
                        label: model::TrackLabel::Bass,
                        waveform: generate_waveform(),
                        ..model::Stem::default()
                    },
                    model::Stem {
                        color: color!(0x31B15D, 1.0),
                        label: model::TrackLabel::Melody,
                        waveform: generate_waveform(),
                        ..model::Stem::default()
                    },
                    model::Stem {
                        color: color!(0x4198D7, 1.0),
                        label: model::TrackLabel::Vocals,
                        waveform: generate_waveform(),
                        ..model::Stem::default()
                    },
                ],
                metadata: HashMap::from([
                    (Metadata::Title, MetadataValue::String("A Title".to_owned())),
                    (Metadata::Artist, MetadataValue::String("An Artist".to_owned())),
                    (Metadata::Genre, MetadataValue::String("Techno".to_owned())),
                    (Metadata::TrackNo, MetadataValue::Number(15)),
                ]),
                cover: Some(image::Handle::from_path("../testdata/rocket.png")),
                editing: true,
                selected: true,
                ..Default::default()
            },
        )
    }
}

impl File {
    pub fn new(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let file = Track::new(path)?;
        let id = Uuid::new_v4();
        let metadata = file.tags();
        let cover = file
            .covers()
            .first()
            .map(|p| image::Handle::from_bytes(Bytes::from(p.data.clone())));
        Ok(Self {
            id,
            path: path.to_owned(),
            metadata,
            cover,
            stems: vec![
                Stem {
                    color: color!(0xF40162, 1.),
                    label: TrackLabel::Drums,
                    ..Stem::default()
                },
                Stem {
                    color: color!(0xFF9D0A, 1.),
                    label: TrackLabel::Bass,
                    ..Stem::default()
                },
                Stem {
                    color: color!(0x31B15D, 1.0),
                    label: TrackLabel::Melody,
                    ..Stem::default()
                },
                Stem {
                    color: color!(0x4198D7, 1.0),
                    label: TrackLabel::Vocals,
                    ..Stem::default()
                },
            ],
            ..Default::default()
        })
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn label(&self) -> String {
        match (
            self.metadata.get(&Metadata::Title),
            self.metadata.get(&Metadata::Artist),
        ) {
            (None, None) => self
                .path
                .file_name()
                .map_or("Unknown", |f| f.to_str().unwrap_or("Unknown"))
                .to_owned(),
            (None, Some(artist)) => format!("Unknown - {}", artist.to_string()),
            (Some(title), None) => format!("{} - Unknown", title.to_string()),
            (Some(title), Some(artist)) => {
                format!("{} - {}", title.to_string(), artist.to_string())
            }
        }
    }
    pub fn set_waveforms(&mut self, data: Vec<Vec<(f32, f32)>>) {
        self.stems
            .iter_mut()
            .zip(data)
            .for_each(|(stem, data)| stem.waveform = Some(data));
    }
    pub fn preview_mask(&self) -> u8 {
        self.stems
            .iter()
            .enumerate()
            .map(|(i, stem)| (!stem.muted as u8) << i)
            .sum()
    }
    pub fn is_ready(&self) -> bool {
        self.stems.iter().all(|s| s.waveform.is_some()) && self.rendered.is_some()
    }

    pub fn set_rendered(&mut self, rendered: RenderedFile) {
        self.rendered = Some(rendered);
        self.progress = Some(1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unpack() {
        let float: [u8; 4] = [0xae, 'N' as u8, 0xb0, '=' as u8];
        assert_eq!(f32::from_le_bytes(float), 0.08608756959438324);
    }
}
