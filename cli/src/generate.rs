use indicatif::{ProgressBar, ProgressStyle};
use stemgen::{
    demucs::{Demucs, DemusOpts},
    nistem::{self, NIStem},
    track::Track,
};

use crate::cli::{Cli, GenerateArgs};

pub fn generate(ctx: &Cli, command: &GenerateArgs) -> Result<bool, Box<dyn std::error::Error>> {
    let mut demucs = Demucs::new_from_file(
        &command.model,
        DemusOpts {
            threads: command.thread,
            device: command.device,
        },
    )?;
    let mut has_failure = false;
    let sample_rate: u64 = ctx.sample_rate.into();

    for file in &command.files {
        let filename = file.file_name().unwrap();
        let output_filename = format!(
            "{}.{}",
            filename
                .to_str()
                .map(|s| s.split('.').next().unwrap())
                .unwrap(),
            ctx.ext
        );
        let output_file = command.output.join(output_filename);
        if output_file.exists() {
            if !ctx.force {
                eprintln!(
                    "Cannot proceed with {}: stem file already exist in output directory!",
                    file.display()
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
        let mut read = 0;
        let pb = ProgressBar::new(2 * input.total() as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {percent}% ({eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
        );

        loop {
            let mut buf: Vec<f32> = vec![0f32; 343980 * 2];
            let mut original_packets = Vec::with_capacity(512);
            let mut original_buffer: Vec<f32> = Vec::with_capacity(512);

            let (data, eof) = loop {
                let size = input.read(
                    if matches!(nistem, NIStem::PreservedMaster(..)) {
                        Some(&mut original_packets)
                    } else {
                        None
                    },
                    &mut buf,
                )?;
                read += size;
                if matches!(nistem, NIStem::ConsistentStream(..)) {
                    original_buffer.extend(buf[..size].to_vec());
                }
                if let Some(mut data) = demucs.send(&buf[..size])? {
                    if matches!(nistem, NIStem::ConsistentStream(..)) {
                        data.insert(0, original_buffer);
                    }
                    break (data, false)
                }
                if size != buf.len() {
                    let mut data = demucs.flush()?;
                    if matches!(nistem, NIStem::ConsistentStream(..)) {
                        data.insert(0, original_buffer);
                    }
                    break (data, true);
                }
            };
            pb.set_position(read as u64 / sample_rate);
            match nistem {
                NIStem::PreservedMaster(..) => nistem.write_preserved(original_packets, data)?,
                NIStem::ConsistentStream(..) => nistem.write_consistent(data)?,
            }

            if eof {
                break;
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

    use stemgen::nistem::{Codec, SampleRate};

    use crate::{cli::GenerateArgs, constants::DEFAULT_EXT, generate::generate, Cli, Commands};

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
    }
}
