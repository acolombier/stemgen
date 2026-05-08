use std::{ffi::OsStr, path::PathBuf};

use glob::glob;
use indicatif::{ProgressBar, ProgressStyle};
use stemgen::{
    audio_ops::planar_to_interleaved,
    demucs::{Demucs, DemusOpts},
    nistem::{self, NIStem},
    track::Track,
};

use crate::cli::{Cli, GenerateArgs};

fn split_file_at_dot(file: &OsStr) -> (&OsStr, Option<&OsStr>) {
    let slice = file.as_encoded_bytes();
    if slice == b".." {
        return (file, None);
    }

    let mut current_idx = slice.len();

    while current_idx > 0 {
        match slice[1..current_idx].iter().rposition(|b| *b == b'.') {
            Some(i) =>  {
                let ext = &slice[i+1..].to_ascii_lowercase();
                if !ext.iter().all(|c|(*c >= b'0' && *c <= b'9') || (*c >= b'a' && *c <= b'z') || *c == b'.') {
                    break;
                }
                current_idx = i + 1;
            },
            None => break,
        };
    }

    if current_idx == slice.len() {
        return (file, None)
    }

    let before = &slice[..current_idx];
    let after = &slice[current_idx..];
    unsafe {
        (
            OsStr::from_encoded_bytes_unchecked(before),
            Some(OsStr::from_encoded_bytes_unchecked(after)),
        )
    }
}

pub fn generate(ctx: &Cli, command: &GenerateArgs) -> Result<bool, Box<dyn std::error::Error>> {
    let mut demucs = Demucs::new_from_file(
        &command.model,
        DemusOpts {
            threads: command.thread,
            device: command.device,
            overlap: command.overlap,
            transition_power: command.transition_power,
        },
    )?;
    let mut has_failure = false;

    let mut files: Vec<Result<glob::Paths, glob::PatternError>> = command.files.iter().map(|raw|glob(&raw)).collect();

    if let Some(err) = files.iter().find_map(|r|r.as_ref().err()) {
        return Err(format!("unable to render the glob: {}", err).into())
    }

    let files: Vec<PathBuf> = files.iter_mut().filter_map(|r|r.as_mut().ok()).flatten().filter_map(|r|r.ok()).collect();

    for file in &files {
        let filename = file.file_name().map(split_file_at_dot).and_then(|(before, _after)| Some(before));
        if filename.is_none() {
            eprintln!(
                "Unable to detect filename from {}",
                file.display()
            );
            has_failure |= true;
            continue;
        }
        let filename = filename.unwrap();
        let output_filename = format!(
            "{}.{}",
            filename.display(),
            ctx.ext
        );
        let output_file = command.output.join(output_filename);
        if output_file.exists() {
            if !ctx.force {
                eprintln!(
                    "Cannot proceed with {}: stem file already exist in output directory!",
                    output_file.display()
                );
                has_failure |= true;
                continue;
            }
            std::fs::remove_file(&output_file)?;
        }
        let mut input = Track::new(file)?;
        let mut nistem = if command.preserved_original_as_master {
            NIStem::new_with_preserved_original(&output_file, input.args(), ctx)?
        } else {
            NIStem::new_with_consistent_streams(&output_file, ctx)?
        };
        nistem.clone(file)?;

        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::with_template(
                &format!("{{spinner:.green}} {} [{{wide_bar:.cyan/blue}}] [{{elapsed_precise}}] {{percent}}% ({{eta}})", filename.display()),
            )
            .unwrap()
            .progress_chars("#>-"),
        );

        // Read entire audio file into memory
        let total_samples = input.total_samples() as usize;
        let mut audio_buffer: Vec<f32> = vec![0f32; total_samples];
        let mut original_packets = Vec::with_capacity(512);
        let mut read_offset = 0;

        pb.set_message("Reading audio...");
        while read_offset < audio_buffer.len() {
            let remaining = audio_buffer.len() - read_offset;
            let chunk_size = std::cmp::min(343980 * 2, remaining);
            let size = input.read(
                if matches!(nistem, NIStem::PreservedMaster(..)) {
                    Some(&mut original_packets)
                } else {
                    None
                },
                &mut audio_buffer[read_offset..read_offset + chunk_size],
            )?;

            if size == 0 {
                audio_buffer.truncate(read_offset);
                break;
            }
            read_offset += size;
            pb.set_position((read_offset as u64 * 10) / total_samples as u64);
        }

        // Process with demucs using overlap
        pb.set_message("Processing stems...");
        let stems = demucs.process(&audio_buffer, |current, total| {
            let progress = 10 + (current as u64 * 80) / total as u64;
            pb.set_position(progress);
        })?;
        pb.set_position(90);

        // Write stems
        pb.set_message("Writing output...");

        // Convert stems from planar to interleaved
        let stems_interleaved: Vec<Vec<f32>> = stems.into_iter()
            .map(|[left, right]| planar_to_interleaved(&left, &right))
            .collect();

        match nistem {
            NIStem::PreservedMaster(..) => {
                nistem.write_preserved(original_packets, stems_interleaved)?;
            },
            NIStem::ConsistentStream(..) => {
                // Original audio is already interleaved, prepend it to stems
                let mut data_with_original = vec![audio_buffer];
                data_with_original.extend(stems_interleaved);
                nistem.write_consistent(data_with_original)?;
            }
        }

        pb.finish_with_message(format!("downloaded {}", filename.display()));
        nistem.flush(nistem::Atom {
            stems: [
                nistem::AtomStem {
                    color: ctx.drum_stem_color.to_owned(),
                    name: ctx.drum_stem_label.to_owned(),
                },
                nistem::AtomStem {
                    color: ctx.bass_stem_color.to_owned(),
                    name: ctx.bass_stem_label.to_owned(),
                },
                nistem::AtomStem {
                    color: ctx.other_stem_color.to_owned(),
                    name: ctx.other_stem_label.to_owned(),
                },
                nistem::AtomStem {
                    color: ctx.vocal_stem_color.to_owned(),
                    name: ctx.vocal_stem_label.to_owned(),
                },
            ],
            version: 1,
            ..Default::default()
        })?;
    }
    Ok(has_failure)
}

#[cfg(test)]
mod tests {

    use std::path::Path;

    use stemgen::nistem::{Codec, SampleRate};

    use crate::{cli::GenerateArgs, constants::DEFAULT_EXT, generate::{generate, split_file_at_dot}, Cli, Commands};

    #[test]
    fn test_generate_command() {
        let ctx = Cli {
            force: true,
            verbose: false,
            codec: Codec::OPUS,
            sample_rate: SampleRate::Hz48000,
            ext: DEFAULT_EXT.to_owned(),
            command: Commands::Generate(GenerateArgs {
                files: vec!["../testdata/Oddchap - Sound 104.mp3".into()],
                output: "..".into(),
                preserved_original_as_master: false,
                ..Default::default()
            }),
            ..Default::default()
        };
        if let Commands::Generate(command) = &ctx.command {
            let result = generate(&ctx, command);
            assert!(
                matches!(result, Ok(false)),
                "Expected value to match pattern, but got: {result:?}"
            );
        } else {
            unreachable!("unexpected command value")
        }

        let ctx = Cli {
            force: true,
            verbose: false,
            codec: Codec::OPUS,
            sample_rate: SampleRate::Hz48000,
            ext: DEFAULT_EXT.to_owned(),
            command: Commands::Generate(GenerateArgs {
                files: vec!["../**/*.mp3".into()],
                output: "..".into(),
                preserved_original_as_master: false,
                ..Default::default()
            }),
            ..Default::default()
        };
        if let Commands::Generate(command) = &ctx.command {
            let result = generate(&ctx, command);
            assert!(
                matches!(result, Ok(false)),
                "Expected value to match pattern, but got: {result:?}"
            );
        } else {
            unreachable!("unexpected command value")
        }
    }

    #[test]
    fn test_can_get_file_name(){
        let file_name = Path::new("Flo Rida - Low (feat. T-Pain).ogg").file_name().map(split_file_at_dot).and_then(|(before, _after)| Some(before)).unwrap().to_str().unwrap().to_owned();
        assert_eq!(&file_name, "Flo Rida - Low (feat. T-Pain)");

        let file_name = Path::new("Flo Rida - Low (feat. T-Pain).stem.mp4").file_name().map(split_file_at_dot).and_then(|(before, _after)| Some(before)).unwrap().to_str().unwrap().to_owned();
        assert_eq!(&file_name, "Flo Rida - Low (feat. T-Pain)");
    }
}
