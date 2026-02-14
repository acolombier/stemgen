use std::path::{Component, PathBuf};

use clap::{builder::ValueParser, value_parser, ArgAction, Parser, Subcommand};
use stemgen::{
    constant::{DEFAULT_MODEL, STEM_DEFAULT_COLOR, STEM_DEFAULT_LABEL}, demucs::{Device, Model}, nistem::{Codec, Color, SampleRate}
};

use crate::constants::*;

fn parse_color(value: &str) -> Result<Color, String> {
    value.try_into()
}

fn parse_codec(value: &str) -> Result<Codec, String> {
    value.try_into()
}

fn parse_samplerate(value: &str) -> Result<SampleRate, String> {
    value.try_into()
}

fn parse_device(value: &str) -> Result<Device, String> {
    value.try_into()
}

fn parse_model(value: &str) -> Result<Model, String> {
    value.try_into()
}

/// A fictional versioning CLI
#[derive(Debug, Parser, Default)] // requires `derive` feature
#[command(name = "stemgen")]
#[command(version, about = "Generate a NI STEM file out of an audio stereo file.", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, help = "Proceed even if the output file already exists", default_value_t = false, action = ArgAction::SetTrue, global = true)]
    pub force: bool,
    #[arg(long, help = "Display verbose information which may be useful for debugging", default_value_t = false, action = ArgAction::SetTrue, global = true)]
    pub verbose: bool,
    #[arg(short, long, help = "The codec to use for the stem stream stored in the output MP4", value_enum, value_parser = ValueParser::new(parse_codec), default_value = "aac", global = true)]
    pub codec: Codec,
    #[arg(short, long, help = "The sample rate to use for the output", value_enum, value_parser = ValueParser::new(parse_samplerate), default_value = "44100", global = true)]
    pub sample_rate: SampleRate,
    #[arg(long, help = "Custom label for the drum stem (the first one)", value_name = "LABEL", default_value_t = STEM_DEFAULT_LABEL[0].to_owned(), global = true)]
    pub drum_stem_label: String,
    #[arg(long, help = "Custom label for the bass stem (the second one)", value_name = "LABEL", default_value_t = STEM_DEFAULT_LABEL[1].to_owned(), global = true)]
    pub bass_stem_label: String,
    #[arg(long, help = "Custom label for the other stem (the third one)", value_name = "LABEL", default_value_t = STEM_DEFAULT_LABEL[2].to_owned(), global = true)]
    pub other_stem_label: String,
    #[arg(long, help = "Custom label for the vocal stem (the fourth and last one)", value_name = "LABEL", default_value_t = STEM_DEFAULT_LABEL[3].to_owned(), global = true)]
    pub vocal_stem_label: String,
    #[arg(long, help = "Custom color for the drum stem (the first one)", value_parser = ValueParser::new(parse_color), value_name = "HEX_COLOR", default_value_t = STEM_DEFAULT_COLOR[0].to_owned(), global = true)]
    pub drum_stem_color: Color,
    #[arg(long, help = "Custom color for the bass stem (the second one)", value_parser = ValueParser::new(parse_color), value_name = "HEX_COLOR", default_value_t = STEM_DEFAULT_COLOR[1].to_owned(), global = true)]
    pub bass_stem_color: Color,
    #[arg(long, help = "Custom color for the other stem (the third one)", value_parser = ValueParser::new(parse_color), value_name = "HEX_COLOR", default_value_t = STEM_DEFAULT_COLOR[2].to_owned(), global = true)]
    pub other_stem_color: Color,
    #[arg(long, help = "Custom color for the vocal stem (the fourth and last one)", value_parser = ValueParser::new(parse_color), value_name = "HEX_COLOR", default_value_t = STEM_DEFAULT_COLOR[3].to_owned(), global = true)]
    pub vocal_stem_color: Color,
    #[arg(short, long, help = "Extension for the STEM file", value_name = "EXT", default_value_t = DEFAULT_EXT.to_owned(), global = true)]
    pub ext: String,
}

impl From<&'_ Cli> for (ffmpeg_next::codec::Id, i32) {
    fn from(val: &'_ Cli) -> Self {
        (val.codec.into(), val.sample_rate.into())
    }
}

#[derive(Debug, Parser)]
pub struct CreateArgs {
    #[arg(required = true)]
    pub output: PathBuf,
    #[arg(long, required = true)]
    pub mastered: PathBuf,
    #[arg(long, required = true)]
    pub drum: PathBuf,
    #[arg(long, required = true)]
    pub bass: PathBuf,
    #[arg(long, required = true)]
    pub other: PathBuf,
    #[arg(long, required = true)]
    pub vocal: PathBuf,
    #[arg(long, default_value_t = true)]
    pub copy_id3tags_from_mastered: bool,
}

#[derive(Debug, Parser, Default, Copy, Clone)]
pub enum DeleteOriginal {
    #[default]
    No,
    #[cfg(unix)]
    Symlink,
    Yes
}

fn diff_paths(old: &PathBuf, new: &PathBuf) -> Result<PathBuf, Box<dyn std::error::Error>>
{
    let mut ita = new.parent().ok_or("expected a parented target")?.components();
    let mut itb = old.parent().ok_or("expected a parented target")?.components();
    let mut comps: Vec<Component> = vec![];

    // ./foo and foo are the same
    if let Some(Component::CurDir) = ita.clone().next() {
        ita.next();
    }
    if let Some(Component::CurDir) = itb.clone().next() {
        itb.next();
    }

    loop {
        match (ita.next(), itb.next()) {
            (None, None) => break,
            (Some(a), None) => {
                comps.push(a);
                comps.extend(ita.by_ref());
                break;
            }
            (None, _) => comps.push(Component::ParentDir),
            (Some(a), Some(b)) if comps.is_empty() && a == b => (),
            (Some(a), Some(b)) if b == Component::CurDir => comps.push(a),
            (Some(_), Some(b)) if b == Component::ParentDir => return Err("unexpected parent dir".into()),
            (Some(a), Some(_)) => {
                comps.push(Component::ParentDir);
                for _ in itb {
                    comps.push(Component::ParentDir);
                }
                comps.push(a);
                comps.extend(ita.by_ref());
                break;
            }
        }
    }
    let rel: PathBuf = comps.iter().map(|c| c.as_os_str()).collect();
    let filename = new.file_name().ok_or("missing filename in target")?;
    Ok(rel.join(filename))
}

impl DeleteOriginal {
    pub(crate) fn proceed(&self, original: &PathBuf, new: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            DeleteOriginal::No => Ok(()),
            #[cfg(unix)]
            DeleteOriginal::Symlink => {
                use std::fs::canonicalize;

                let new = canonicalize(new)?;
                let original = canonicalize(original)?;
                std::fs::remove_file(&original)?;

                std::os::unix::fs::symlink(diff_paths(&original, &new)?, original)
            },
            DeleteOriginal::Yes =>
                std::fs::remove_file(original),
        }.map_err(|e|e.into())
    }
}

fn parse_deleteoriginal(value: &str) -> Result<DeleteOriginal, String> {
    value.try_into()
}

impl TryFrom<&str> for DeleteOriginal {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_ascii_lowercase().as_str() {
            "no" => Ok(Self::No),
            "symlink" => Ok(Self::Symlink),
            "yes" => Ok(Self::Yes),
            _ => Err("unsupported value".to_owned()),
        }
    }
}

#[derive(Debug, Parser, Default)]
pub struct GenerateArgs {
    #[arg(num_args = 1.., value_name = "FILES", help = "path(s) to a file supported by the FFmpeg codec available on your machine. Advanced glob pattern can be used such as '~/Music/**/*.mp3'", required = true)]
    pub files: Vec<String>,
    #[arg(value_name = "OUTPUT", help = "path to an existing directory where to store the generated STEM file(s)", value_parser = value_parser!(PathBuf), required = true)]
    pub output: PathBuf,
    #[arg(long, value_name = "DEVICE", help = "Device for the demucs model inference", value_parser = ValueParser::new(parse_device), default_value_t = Device::CPU)]
    pub device: Device,
    #[arg(long, value_name = "PATH", help = "The model to use with demucs. Default to htdemucs fine-trained", value_parser = ValueParser::new(parse_model), default_value = DEFAULT_MODEL)]
    pub model: Model,
    #[arg(
        long,
        value_name = "INTEGER",
        help = "The number of jobs to use for demucs.",
        default_value_t = 4
    )]
    pub thread: usize,
    #[arg(long, default_value_t = false)]
    pub preserve_original_as_master: bool,
    #[arg(long, value_enum, value_parser = ValueParser::new(parse_deleteoriginal), default_value = "no")]
    pub delete_original: DeleteOriginal,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(arg_required_else_help = true)]
    Generate(GenerateArgs),
    #[command(arg_required_else_help = true)]
    Create(CreateArgs),
}

impl Default for Commands {
    fn default() -> Self {
        Self::Generate(GenerateArgs::default())
    }
}

pub fn prepare_ffmpeg(ctx: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    ffmpeg_next::init()?;
    if ctx.verbose {
        ffmpeg_next::log::set_level(ffmpeg_next::log::Level::Trace);
    } else {
        ffmpeg_next::log::set_level(ffmpeg_next::log::Level::Fatal);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::CommandFactory;

    use crate::{cli::diff_paths, Cli};

    #[test]
    fn verify_cmd() {
        Cli::command().debug_assert();
    }

    #[test]
    fn test_can_resolve_relative_to_original(){
        let original = PathBuf::from("../Manau - La Tribu de Dana.mp3");
        let new = PathBuf::from("../Manau - La Tribu de Dana.stem.mp4");

        assert_eq!(diff_paths(&original, &new).unwrap(), PathBuf::from("Manau - La Tribu de Dana.stem.mp4"));

        let original = PathBuf::from("./Manau - La Tribu de Dana.mp3");
        let new = PathBuf::from("../Manau - La Tribu de Dana.stem.mp4");

        assert_eq!(diff_paths(&original, &new).unwrap(), PathBuf::from("../Manau - La Tribu de Dana.stem.mp4"));

        let original = PathBuf::from("/mnt/Stereo/Manau - La Tribu de Dana.mp3");
        let new = PathBuf::from("/mnt/Stem/Manau - La Tribu de Dana.stem.mp4");

        assert_eq!(diff_paths(&original, &new).unwrap(), PathBuf::from("../Stem/Manau - La Tribu de Dana.stem.mp4"));
    }
}
