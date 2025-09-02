use std::{collections::HashMap, fmt, path::PathBuf};

use ffmpeg_next::{
    codec::{self, Compliance}, encoder::{self}, ffi::AVFMT_FLAG_GENPTS, format::{self, context}, frame::Audio, software::resampling, ChannelLayout, Packet, Rational
};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, Unexpected, Visitor},
};
use taglib::AttachedPicture;

use crate::constant::{Metadata, MetadataValue, STEM_DEFAULT_COLOR, STEM_DEFAULT_LABEL};

#[derive(Debug, Clone, Default, Copy)]
pub enum Codec {
    #[default]
    AAC,
    ALAC,
    FLAC,
    OPUS,
}

#[derive(Debug, Clone, Default, Copy)]
pub enum SampleRate {
    #[default]
    Hz44100,
    Hz48000,
}

impl From<String> for Codec {
    fn from(value: String) -> Self {
        match value.as_str() {
            "alac" => Codec::ALAC,
            "flac" => Codec::FLAC,
            "opus" => Codec::OPUS,
            _ => Codec::AAC,
        }
    }
}

impl From<Codec> for codec::Id {
    fn from(val: Codec) -> Self {
        match val {
            Codec::ALAC => codec::Id::ALAC,
            Codec::FLAC => codec::Id::FLAC,
            Codec::OPUS => codec::Id::OPUS,
            Codec::AAC => codec::Id::AAC,
        }
    }
}

impl std::fmt::Display for Codec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Codec::AAC => write!(f, "aac"),
            Codec::ALAC => write!(f, "alac"),
            Codec::FLAC => write!(f, "flac"),
            Codec::OPUS => write!(f, "opus"),
        }
    }
}

impl From<String> for SampleRate {
    fn from(value: String) -> Self {
        match value.as_str() {
            "48000" => SampleRate::Hz48000,
            _ => SampleRate::Hz44100,
        }
    }
}

impl From<SampleRate> for i32 {
    fn from(val: SampleRate) -> Self {
        match val {
            SampleRate::Hz48000 => 48000,
            SampleRate::Hz44100 => 44100,
        }
    }
}

impl From<SampleRate> for u64 {
    fn from(val: SampleRate) -> Self {
        match val {
            SampleRate::Hz48000 => 48000,
            SampleRate::Hz44100 => 44100,
        }
    }
}

impl std::fmt::Display for SampleRate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SampleRate::Hz48000 => write!(f, "48000 Hz"),
            SampleRate::Hz44100 => write!(f, "44100 Hz"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Color(pub i32);

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ColorVisitor;

        impl<'de> Visitor<'de> for ColorVisitor {
            type Value = Color;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("string with a valid Color")
            }
            fn visit_str<E>(self, value: &str) -> Result<Color, E>
            where
                E: de::Error,
            {
                Color::try_from(value).map_err(|_| {
                    de::Error::invalid_value(
                        Unexpected::Str(value),
                        &"color in hexadecimal with a leading hash",
                    )
                })
            }
        }

        deserializer.deserialize_str(ColorVisitor)
    }
}

impl Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = &format!("{:#08x}", self.0)[2..].to_ascii_uppercase();
        write!(f, "#{repr}")
    }
}

impl TryFrom<&str> for Color {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        i32::from_str_radix(&value[1..], 16)
            .map(Self)
            .map_err(|_| "invalid color format".to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtomStem {
    pub color: Color,
    pub name: String,
}

impl AtomStem {
    pub fn new(name: String, color: Color) -> Self {
        Self { color, name }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtomMasteringDSPCompressor {
    pub enabled: bool,
    pub ratio: i32,
    pub output_gain: i32,
    pub release: f32,
    pub attack: f32,
    pub input_gain: i32,
    pub threshold: i32,
    pub hp_cutoff: i32,
    pub dry_wet: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtomMasteringDSPLimiter {
    pub enabled: bool,
    pub release: f32,
    pub threshold: i32,
    pub ceiling: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtomMasteringDSP {
    pub compressor: AtomMasteringDSPCompressor,
    pub limiter: AtomMasteringDSPLimiter,
}

impl Default for AtomMasteringDSP {
    fn default() -> Self {
        Self {
            compressor: AtomMasteringDSPCompressor {
                enabled: false,
                ratio: 10,
                output_gain: 0,
                release: 1.0,
                attack: 0.0001,
                input_gain: 0,
                threshold: 0,
                hp_cutoff: 20,
                dry_wet: 100,
            },
            limiter: AtomMasteringDSPLimiter {
                enabled: false,
                release: 1.0,
                threshold: 0,
                ceiling: 0,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Atom {
    pub stems: [AtomStem; 4],
    pub mastering_dsp: AtomMasteringDSP,
    pub version: i32,
}

impl Default for Atom {
    fn default() -> Self {
        Self {
            stems: [
                AtomStem::new(
                    STEM_DEFAULT_LABEL[0].to_owned(),
                    STEM_DEFAULT_COLOR[0].to_owned(),
                ),
                AtomStem::new(
                    STEM_DEFAULT_LABEL[1].to_owned(),
                    STEM_DEFAULT_COLOR[1].to_owned(),
                ),
                AtomStem::new(
                    STEM_DEFAULT_LABEL[2].to_owned(),
                    STEM_DEFAULT_COLOR[2].to_owned(),
                ),
                AtomStem::new(
                    STEM_DEFAULT_LABEL[3].to_owned(),
                    STEM_DEFAULT_COLOR[3].to_owned(),
                ),
            ],
            mastering_dsp: Default::default(),
            version: 1,
        }
    }
}

pub struct Inner {
    path: PathBuf,
    ctx: context::Output,
    idx_encoders: Vec<(usize, encoder::Audio, resampling::Context, usize)>,
    overrun: Vec<Vec<f32>>,
    metadata: HashMap<Metadata, MetadataValue>,
    cover: Vec<AttachedPicture>,
}

pub enum NIStem {
    PreservedMaster(Inner, (usize, Rational)),
    ConsistentStream(Inner)
}

impl NIStem {
    pub fn new_with_preserved_original<O: Into<(codec::Parameters, Rational)>, S: Into<(codec::Id, i32)>>(
        path: &PathBuf,
        original: O,
        stem: S,
    ) -> Result<Self, ffmpeg_next::Error> {
        ffmpeg_next::init()?;
        let original = original.into();
        let stem = stem.into();
        let mut ctx = format::output(&path)?;
        unsafe {
            (*ctx.as_mut_ptr()).strict_std_compliance = -2;
        }
        let mut ost = ctx.add_stream(original.0.id())?;
        ost.set_parameters(original.0);
        // We need to set codec_tag to 0 lest we run into incompatible codec tag
        // issues when muxing into a different container format. Unfortunately
        // there's no high level API to do this (yet).
        unsafe {
            (*ost.parameters().as_mut_ptr()).codec_tag = 0;
        }
        let original = (ost.index(), original.1);

        let codec = encoder::find(stem.0).ok_or(ffmpeg_next::Error::InvalidData)?;
        let mut idx_encoders = Vec::new();
        let mut formats = codec
            .audio()?
            .formats()
            .ok_or(ffmpeg_next::Error::InvalidData)?;
        let format = formats.next().ok_or(ffmpeg_next::Error::InvalidData)?;

        for _ in 0..4 {
            let stream = Self::add_stream(&mut ctx, codec, format, stem.1)?;
            idx_encoders.push(stream);
        }

        ctx.write_header()?;

        Ok(Self::PreservedMaster(
            Inner {
                ctx,
                path: path.clone(),
                idx_encoders,
                overrun: vec![Default::default(); 4],
                metadata: Default::default(),
                cover: Default::default()
            },
            original,
        ))
    }

    pub fn new_with_consistent_streams<S: Into<(codec::Id, i32)>>(
        path: &PathBuf,
        stem: S,
    ) -> Result<Self, ffmpeg_next::Error> {
        ffmpeg_next::init()?;
        let stem = stem.into();
        // -fflags +genpts ?
        let mut ctx = format::output(&path)?;
        unsafe {
            // Needed for OPUS and FLAC?
            (*ctx.as_mut_ptr()).strict_std_compliance = -2;
            (*ctx.as_mut_ptr()).flags |= AVFMT_FLAG_GENPTS;
        }

        let codec = encoder::find(stem.0).ok_or(ffmpeg_next::Error::InvalidData)?;
        let mut idx_encoders = Vec::new();
        let mut formats = codec
            .audio()?
            .formats()
            .ok_or(ffmpeg_next::Error::InvalidData)?;
        let format = formats.next().ok_or(ffmpeg_next::Error::InvalidData)?;

        for _ in 0..5 {
            let stream = Self::add_stream(&mut ctx, codec, format, stem.1)?;
            idx_encoders.push(stream);
        }

        ctx.write_header()?;

        Ok(Self::ConsistentStream(
            Inner {
                ctx,
                path: path.clone(),
                idx_encoders,
                overrun: vec![Default::default(); 5],
                metadata: Default::default(),
                cover: Default::default()
            }
        ))
    }

    fn add_stream(
        ctx: &mut context::Output,
        codec: ffmpeg_next::Codec,
        format: format::Sample,
        sample_rate: i32,
    ) -> Result<(usize, encoder::Audio, resampling::Context, usize), ffmpeg_next::Error> {
        let mut encoder = codec::context::Context::new()
            .encoder()
            .audio()?;
        encoder.compliance(Compliance::Experimental);
        encoder.set_flags(codec::flag::Flags::GLOBAL_HEADER);
        encoder.set_rate(sample_rate);
        encoder.set_channel_layout(ChannelLayout::STEREO);
        encoder.set_format(format);

        unsafe {
            (*encoder.as_mut_ptr()).frame_size = 1024;
        }
        let mut ost = ctx.add_stream(codec)?;
        let encoder = encoder.open_as(codec)?;
        ost.set_parameters(&encoder);
        let resampler = resampling::Context::get(
            format::Sample::F32(format::sample::Type::Packed),
            ffmpeg_next::ChannelLayout::STEREO,
            sample_rate as u32,
            encoder.format(),
            encoder.channel_layout(),
            encoder.rate(),
        )?;
        Ok((ost.index(), encoder, resampler, 0))
    }

    pub fn metadata(&self, key: &Metadata) -> Option<&MetadataValue> {
        match self {
            NIStem::PreservedMaster(inner, _) | NIStem::ConsistentStream(inner) => inner.metadata.get(key),
        }

    }
    pub fn set_metadata(&mut self, key: Metadata, value: MetadataValue) {
        match self {
            NIStem::PreservedMaster(inner, _) | NIStem::ConsistentStream(inner) => inner.metadata.insert(key, value)
        };
    }
    pub fn clone(&mut self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let tagfile = taglib::File::new(path).map_err(|e| format!("{e:?}"))?;
        let metadata = match tagfile.tag() {
            Ok(tags) => {
                let mut metadata = HashMap::new();
                if let Some(value) = tags.title() {
                    metadata.insert(Metadata::Title, MetadataValue::String(value));
                }
                if let Some(value) = tags.artist() {
                    metadata.insert(Metadata::Artist, MetadataValue::String(value));
                }
                if let Some(value) = tags.album() {
                    metadata.insert(Metadata::Release, MetadataValue::String(value));
                }
                if let Some(value) = tags.comment() {
                    metadata.insert(Metadata::Label, MetadataValue::String(value));
                }
                if let Some(value) = tags.genre() {
                    metadata.insert(Metadata::Genre, MetadataValue::String(value));
                }
                if let Some(value) = tags.track() {
                    metadata.insert(Metadata::TrackNo, MetadataValue::Number(value));
                }
                metadata
            }
            Err(_) => HashMap::new(),
        };
        let cover = tagfile.pictures()?;

        match self {
            NIStem::PreservedMaster(inner, _) | NIStem::ConsistentStream(inner) => {
                inner.metadata = metadata;
                inner.cover = cover;
            }
        };

        Ok(())
    }
    pub fn write_preserved(
        &mut self,
        original: impl IntoIterator<Item = Packet>,
        stems: Vec<Vec<f32>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (inner, original_params) = match self {
            NIStem::PreservedMaster(inner, original) =>Ok((inner, original)),
            _ => Err("cannot write original packet in consistent stem"),
        }?;

        for mut packet in original.into_iter() {
            packet.rescale_ts(
                original_params.1,
                inner.ctx.stream(original_params.0).unwrap().time_base(),
            );
            packet.set_stream(original_params.0);
            packet.write(&mut inner.ctx)?;
        }
        Self::write_streams(inner, stems)
    }
    pub fn write_consistent(
        &mut self,
        stems: Vec<Vec<f32>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let inner = match self {
            NIStem::ConsistentStream(inner) =>Ok(inner),
            _ => Err("cannot write consistent when preserving the originla"),
        }?;
        Self::write_streams(inner, stems)
    }

    fn write_streams(
        inner: &mut Inner,
        stems: Vec<Vec<f32>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if stems.len() != inner.idx_encoders.len() {
            return Err("unexpected buffer count".into());
        }
        for (stream_idx, ((idx, encoder, resampler, timestamp), mut frames)) in inner.idx_encoders.iter_mut().zip(stems).enumerate() {
            let frame_size = 2 * encoder.frame_size() as usize;
            if !inner.overrun[stream_idx].is_empty(){
                frames = {
                    let mut v = inner.overrun[stream_idx].clone();
                    v.extend(frames);
                    v
                };
                inner.overrun[stream_idx].clear();
            }
            if frames.len() % frame_size != 0 {
                let new_len = frames.len() - frames.len() % frame_size;
                inner.overrun[stream_idx].extend_from_slice(&frames[new_len..]);
                frames.resize(frames.len() - frames.len() % frame_size, 0.0);
            }
            for chunk in frames.chunks(frame_size) {
                let mut frame = Audio::new(
                    format::Sample::F32(format::sample::Type::Packed),
                    chunk.len(),
                    ffmpeg_next::ChannelLayout::STEREO,
                );
                frame.set_rate(encoder.rate());
                frame.set_pts(Some(*timestamp as i64));
                *timestamp += chunk.len();
                frame.plane_mut(0).copy_from_slice(chunk);
                frame.set_samples(chunk.len()/2);
                let mut resampled = Audio::empty();
                resampler.run(&frame, &mut resampled)?;
                encoder.send_frame(&resampled)?;
                let mut encoded: Packet = Packet::empty();
                while encoder.receive_packet(&mut encoded).is_ok() {
                    encoded.set_stream(*idx);
                    encoded.write(&mut inner.ctx)?;
                }
            }
        }
        Ok(())
    }

    pub fn flush(self, manifest: Atom) -> Result<(), Box<dyn std::error::Error>> {
        let mut inner = match self {
            NIStem::PreservedMaster(inner, _) | NIStem::ConsistentStream(inner) => inner
        };

        for (stream_idx, (idx, encoder, resampler, timestamp)) in inner.idx_encoders.iter_mut().enumerate() {
            if !inner.overrun[stream_idx].is_empty(){
                let chunk = &inner.overrun[stream_idx];
                let mut frame = Audio::new(
                    format::Sample::F32(format::sample::Type::Packed),
                    chunk.len(),
                    ffmpeg_next::ChannelLayout::STEREO,
                );
                frame.set_rate(encoder.rate());
                frame.set_pts(Some(*timestamp as i64));
                *timestamp += chunk.len();
                frame.plane_mut(0).copy_from_slice(chunk);
                frame.set_samples(chunk.len()/2);
                let mut resampled = Audio::empty();
                resampler.run(&frame, &mut resampled)?;
                encoder.send_frame(&resampled)?;
                inner.overrun[stream_idx].clear();
            }
            if resampler.delay().is_some() {
                let mut resampled = Audio::empty();
                resampler.flush(&mut resampled)?;
                encoder.send_frame(&resampled)?;
            }
            encoder.send_eof()?;
            let mut encoded = Packet::empty();
            while encoder.receive_packet(&mut encoded).is_ok() {
                if unsafe { encoded.is_empty() } {
                    continue;
                }
                encoded.set_stream(*idx);
                encoded.write(&mut inner.ctx)?;
            }
        }
        inner.ctx.write_trailer()?;

        let mut file = taglib::File::new(inner.path).map_err(|e| format!("{e:?}"))?;

        file.set_pictures(inner.cover)?;

        file.set_stem(Some(serde_json::to_string(&manifest)?))?;

        let mut tag = file.tag().map_err(|e| format!("{e:?}"))?;

        for (key, value) in inner.metadata.iter() {
            match (key, value) {
                (Metadata::Title, MetadataValue::String(value)) => {
                    tag.set_title(value);
                    Ok(())
                }
                (Metadata::Artist, MetadataValue::String(value)) => {
                    tag.set_artist(value);
                    Ok(())
                }
                (Metadata::Release, MetadataValue::String(value)) => {
                    tag.set_album(value);
                    Ok(())
                }
                (Metadata::Label, MetadataValue::String(value)) => {
                    tag.set_comment(value);
                    Ok(())
                }
                (Metadata::TrackNo, MetadataValue::Number(value)) => {
                    tag.set_track(*value);
                    Ok(())
                }
                (Metadata::Genre, MetadataValue::String(value)) => {
                    tag.set_genre(value);
                    Ok(())
                }
                _ => Err("unsupported tag format"),
            }?;
        }
        if !file.save() {
            Err("unable to save file".into())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ffmpeg_next::codec;

    use crate::{
        nistem::{Atom, Color, NIStem},
        track::Track,
    };

    #[test]
    fn test_color_from_string() {
        let color: Color = "#FF00FF".try_into().unwrap();
        assert_eq!(color.0, 0xff00ff);
    }

    #[test]
    fn test_color_to_string() {
        let color = Color(0xff00ff);
        assert_eq!(color.to_string(), "#FF00FF");
    }

    #[test]
    fn test_stem_atom_can_render() {
        let manifest = Atom::default();
        let atom = serde_json::to_string(&manifest);
        assert!(
            atom.is_ok(),
            "Expected value to match pattern, but got: {atom:?}"
        );
        let atom = atom.unwrap();
        let expected =  "{\"stems\":[{\"color\":\"#009E73\",\"name\":\"Drums\"},{\"color\":\"#D55E00\",\"name\":\"Bass\"},{\"color\":\"#CC79A7\",\"name\":\"Other\"},{\"color\":\"#56B4E9\",\"name\":\"Vocals\"}],\"mastering_dsp\":{\"compressor\":{\"enabled\":false,\"ratio\":10,\"output_gain\":0,\"release\":1.0,\"attack\":0.0001,\"input_gain\":0,\"threshold\":0,\"hp_cutoff\":20,\"dry_wet\":100},\"limiter\":{\"enabled\":false,\"release\":1.0,\"threshold\":0,\"ceiling\":0}},\"version\":1}";
        assert_eq!(atom, expected);
    }

    #[test]
    fn test_ensure_id3() {
        let output_filename = generate_test_file("test_ensure_id3");

        let file = taglib::File::new(&output_filename).unwrap();

        let prop = file.audioproperties().unwrap();

        assert_eq!(prop.length(), 5);

        let metadata = file.tag().unwrap();

        assert_eq!(metadata.title(), Some("Sound 104".to_owned()));
        assert_eq!(metadata.artist(), Some("Odd Chap".to_owned()));
        assert_eq!(metadata.comment(), Some("Spotify - https://open.spotify.com/track/2481amjn69XrXmgFIvBmbD?si=c5a62dce53744656\n\
            Bandcamp - https://oddchap.bandcamp.com/album/sound-104\n\
            Apple - https://music.apple.com/nz/album/sound-104/1691470758?i=1691470760\n\
            Amazon - https://music.amazon.com.au/albums/B0C7BW18ZG\n\
            \nOH YEAHHHH! It's finally here, I was originally going to keep the \"sound 10_\" \
            series to 3 parts with the exception of doing number 4 only if I found the perfect \
            audio dialogue sample to use. And well I found it and here we are! Sound 104, little \
            less heavy than the other in the series but ive upped the funkiness!".to_owned()));
        assert_eq!(metadata.genre(), Some("Electro Swing".to_owned()));

        std::fs::remove_file(&output_filename).unwrap();
    }

    #[test]
    fn test_stem_manifest() {
        let output_filename = generate_test_file("test_stem_manifest");

        let file = taglib::File::new(&output_filename).unwrap();
        let properties = file.complex_property_keys().unwrap();
        assert_eq!(properties, vec!["PICTURE", "STEM"]);

        let manifest = file.stem().unwrap().unwrap();

        let stem: Atom = serde_json::from_str(&manifest).unwrap();

        assert_eq!(stem, Atom::default());

        std::fs::remove_file(&output_filename).unwrap();
    }

    fn generate_test_file(name: &str) -> PathBuf {
        ffmpeg_next::log::set_level(ffmpeg_next::log::Level::Fatal);
        let mut input = Track::new(&"./testdata/Oddchap - Sound 104.mp3".into()).unwrap();
        let mut packets = Vec::with_capacity(2048);
        let mut buf = vec![0.0f32; 1024 * 1024];
        // let mut buf = vec![0.0f32; 1024 * 1024 * 100];
        // assert!(!matches!(input.read(&mut packets, &mut buf), Ok(size) if size == buf.len()));
        input.read(Some(&mut packets), &mut buf).unwrap();
        let output_filename = std::env::temp_dir().join(format!("{name}.stem.mp4"));
        if output_filename.exists() {
            std::fs::remove_file(&output_filename).unwrap();
        }
        let mut output = NIStem::new_with_preserved_original(
            &output_filename,
            input.args(),
            (codec::Id::AAC, 44100),
        )
        .unwrap();
        output
            .clone(&"./testdata/Oddchap - Sound 104.mp3".into())
            .unwrap();
        output
            .write_preserved(packets, vec![buf.clone(), buf.clone(), buf.clone(), buf])
            .unwrap();
        output.flush(Atom::default()).unwrap();

        output_filename
    }

    #[test]
    fn test_ensure_coverart() {
        let output_filename = generate_test_file("test_ensure_coverart");

        let file = taglib::File::new(&output_filename).unwrap();
        let pictures = file.pictures().unwrap();
        let data: Vec<u8> = std::fs::read("./testdata/rocket.png").unwrap();

        assert_eq!(pictures.len(), 1);
        assert_eq!(pictures[0].data.len(), data.len());
        for (i, (a, b)) in data.iter().zip(&pictures[0].data).enumerate() {
            assert_eq!(*a, *b, "Mismatching byte at {i}");
        }

        std::fs::remove_file(&output_filename).unwrap();
    }

    #[test]
    fn test_can_generate_aac() {
        ffmpeg_next::log::set_level(ffmpeg_next::log::Level::Trace);
        let mut buf = vec![0.0f32; 44100 * 10 * 2];
        let freq = 220f32;
        for i in 0..buf.len() / 2 {
            buf[2*i] = f32::cos(freq * i as f32 * std::f32::consts::PI / 44100_f32) * 0.15;
            buf[2*i + 1] = f32::cos(freq * i as f32 * std::f32::consts::PI / 44100_f32) * 0.15;
        }

        let output_filename = std::env::temp_dir().join("test_can_generate_aac.stem.mp4".to_string());
        if output_filename.exists() {
            std::fs::remove_file(&output_filename).unwrap();
        }
        let mut output = NIStem::new_with_consistent_streams(
            &output_filename,
            (codec::Id::AAC, 44100),
        )
        .unwrap();
        output
            .clone(&"./testdata/Oddchap - Sound 104.mp3".into())
            .unwrap();
        output
            .write_consistent(vec![buf.clone(), buf.clone(), buf.clone(), buf.clone(), buf])
            .unwrap();
        output.flush(Atom::default()).unwrap();


        let file = taglib::File::new(&output_filename).unwrap();

        assert_eq!(file.pictures().unwrap().len(), 1);

        let prop = file.audioproperties().unwrap();

        assert_eq!(prop.length(), 10);
        assert_eq!(prop.samplerate(), 44100);
        assert_eq!(prop.channels(), 2);

        let metadata = file.tag().unwrap();

        assert_eq!(metadata.title(), Some("Sound 104".to_owned()));
        assert_eq!(metadata.artist(), Some("Odd Chap".to_owned()));
        assert_eq!(metadata.comment(), Some("Spotify - https://open.spotify.com/track/2481amjn69XrXmgFIvBmbD?si=c5a62dce53744656\n\
            Bandcamp - https://oddchap.bandcamp.com/album/sound-104\n\
            Apple - https://music.apple.com/nz/album/sound-104/1691470758?i=1691470760\n\
            Amazon - https://music.amazon.com.au/albums/B0C7BW18ZG\n\
            \nOH YEAHHHH! It's finally here, I was originally going to keep the \"sound 10_\" \
            series to 3 parts with the exception of doing number 4 only if I found the perfect \
            audio dialogue sample to use. And well I found it and here we are! Sound 104, little \
            less heavy than the other in the series but ive upped the funkiness!".to_owned()));
        assert_eq!(metadata.genre(), Some("Electro Swing".to_owned()));
    }

    #[test]
    fn test_can_generate_alac() {
        ffmpeg_next::log::set_level(ffmpeg_next::log::Level::Trace);
        let mut buf = vec![0.0f32; 44100 * 10 * 2];
        let freq = 220f32;
        for i in 0..buf.len() / 2 {
            buf[2*i] = f32::cos(freq * i as f32 * std::f32::consts::PI / 44100_f32) * 0.15;
            buf[2*i + 1] = f32::cos(freq * i as f32 * std::f32::consts::PI / 44100_f32) * 0.15;
        }

        let output_filename = std::env::temp_dir().join("test_can_generate_alac.stem.mp4".to_string());
        if output_filename.exists() {
            std::fs::remove_file(&output_filename).unwrap();
        }
        let mut output = NIStem::new_with_consistent_streams(
            &output_filename,
            (codec::Id::ALAC, 44100),
        )
        .unwrap();
        output
            .clone(&"./testdata/Oddchap - Sound 104.mp3".into())
            .unwrap();
        output
            .write_consistent(vec![buf.clone(), buf.clone(), buf.clone(), buf.clone(), buf])
            .unwrap();
        output.flush(Atom::default()).unwrap();


        let file = taglib::File::new(&output_filename).unwrap();

        assert_eq!(file.pictures().unwrap().len(), 1);

        let prop = file.audioproperties().unwrap();

        assert_eq!(prop.length(), 10);

        let metadata = file.tag().unwrap();

        assert_eq!(metadata.title(), Some("Sound 104".to_owned()));
        assert_eq!(metadata.artist(), Some("Odd Chap".to_owned()));
        assert_eq!(metadata.comment(), Some("Spotify - https://open.spotify.com/track/2481amjn69XrXmgFIvBmbD?si=c5a62dce53744656\n\
            Bandcamp - https://oddchap.bandcamp.com/album/sound-104\n\
            Apple - https://music.apple.com/nz/album/sound-104/1691470758?i=1691470760\n\
            Amazon - https://music.amazon.com.au/albums/B0C7BW18ZG\n\
            \nOH YEAHHHH! It's finally here, I was originally going to keep the \"sound 10_\" \
            series to 3 parts with the exception of doing number 4 only if I found the perfect \
            audio dialogue sample to use. And well I found it and here we are! Sound 104, little \
            less heavy than the other in the series but ive upped the funkiness!".to_owned()));
        assert_eq!(metadata.genre(), Some("Electro Swing".to_owned()));
    }

    #[test]
    fn test_can_generate_opus() {
        ffmpeg_next::log::set_level(ffmpeg_next::log::Level::Trace);
        let mut buf = vec![0.0f32; 48000 * 10 * 2];
        let freq = 220f32;
        for i in 0..buf.len() / 2 {
            buf[2*i] = f32::cos(freq * i as f32 * std::f32::consts::PI / 48000_f32) * 0.15;
            buf[2*i + 1] = f32::cos(freq * i as f32 * std::f32::consts::PI / 48000_f32) * 0.15;
        }

        let output_filename = std::env::temp_dir().join("test_can_generate_opus.stem.mp4".to_string());
        if output_filename.exists() {
            std::fs::remove_file(&output_filename).unwrap();
        }
        let mut output = NIStem::new_with_consistent_streams(
            &output_filename,
            (codec::Id::OPUS, 48000),
        )
        .unwrap();
        output
            .clone(&"./testdata/Oddchap - Sound 104.mp3".into())
            .unwrap();
        output
            .write_consistent(vec![buf.clone(), buf.clone(), buf.clone(), buf.clone(), buf])
            .unwrap();
        output.flush(Atom::default()).unwrap();


        let file = taglib::File::new(&output_filename).unwrap();

        assert_eq!(file.pictures().unwrap().len(), 1);

        let prop = file.audioproperties().unwrap();

        assert_eq!(prop.length(), 10);

        let metadata = file.tag().unwrap();

        assert_eq!(metadata.title(), Some("Sound 104".to_owned()));
        assert_eq!(metadata.artist(), Some("Odd Chap".to_owned()));
        assert_eq!(metadata.comment(), Some("Spotify - https://open.spotify.com/track/2481amjn69XrXmgFIvBmbD?si=c5a62dce53744656\n\
            Bandcamp - https://oddchap.bandcamp.com/album/sound-104\n\
            Apple - https://music.apple.com/nz/album/sound-104/1691470758?i=1691470760\n\
            Amazon - https://music.amazon.com.au/albums/B0C7BW18ZG\n\
            \nOH YEAHHHH! It's finally here, I was originally going to keep the \"sound 10_\" \
            series to 3 parts with the exception of doing number 4 only if I found the perfect \
            audio dialogue sample to use. And well I found it and here we are! Sound 104, little \
            less heavy than the other in the series but ive upped the funkiness!".to_owned()));
        assert_eq!(metadata.genre(), Some("Electro Swing".to_owned()));
    }

    #[test]
    fn test_can_generate_flac() {
        ffmpeg_next::log::set_level(ffmpeg_next::log::Level::Trace);
        let mut buf = vec![0.0f32; 44100 * 10 * 2];
        let freq = 220f32;
        for i in 0..buf.len() / 2 {
            buf[2*i] = f32::cos(freq * i as f32 * std::f32::consts::PI / 44100_f32) * 0.15;
            buf[2*i + 1] = f32::cos(freq * i as f32 * std::f32::consts::PI / 44100_f32) * 0.15;
        }

        let output_filename = std::env::temp_dir().join("test_can_generate_flac.stem.mp4".to_string());
        if output_filename.exists() {
            std::fs::remove_file(&output_filename).unwrap();
        }
        let mut output = NIStem::new_with_consistent_streams(
            &output_filename,
            (codec::Id::FLAC, 44100),
        )
        .unwrap();
        output
            .clone(&"./testdata/Oddchap - Sound 104.mp3".into())
            .unwrap();
        output
            .write_consistent(vec![buf.clone(), buf.clone(), buf.clone(), buf.clone(), buf])
            .unwrap();
        output.flush(Atom::default()).unwrap();


        let file = taglib::File::new(&output_filename).unwrap();

        assert_eq!(file.pictures().unwrap().len(), 1);

        let prop = file.audioproperties().unwrap();

        assert_eq!(prop.length(), 10);

        let metadata = file.tag().unwrap();

        assert_eq!(metadata.title(), Some("Sound 104".to_owned()));
        assert_eq!(metadata.artist(), Some("Odd Chap".to_owned()));
        assert_eq!(metadata.comment(), Some("Spotify - https://open.spotify.com/track/2481amjn69XrXmgFIvBmbD?si=c5a62dce53744656\n\
            Bandcamp - https://oddchap.bandcamp.com/album/sound-104\n\
            Apple - https://music.apple.com/nz/album/sound-104/1691470758?i=1691470760\n\
            Amazon - https://music.amazon.com.au/albums/B0C7BW18ZG\n\
            \nOH YEAHHHH! It's finally here, I was originally going to keep the \"sound 10_\" \
            series to 3 parts with the exception of doing number 4 only if I found the perfect \
            audio dialogue sample to use. And well I found it and here we are! Sound 104, little \
            less heavy than the other in the series but ive upped the funkiness!".to_owned()));
        assert_eq!(metadata.genre(), Some("Electro Swing".to_owned()));
    }
}
