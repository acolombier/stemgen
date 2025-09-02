
use indicatif::{ProgressBar, ProgressStyle};
use stemgen::{nistem::{self, NIStem}, track::Track};

use crate::cli::{Cli, CreateArgs};

pub fn create(ctx: &Cli, command: &CreateArgs) -> Result<bool, Box<dyn std::error::Error>> {
        let output_file = &command.output;
        if output_file.exists() {
            if !ctx.force {
                eprintln!("Cannot proceed with {}: stem file already exist in output directory!", output_file.display());
                return Ok(true);
            }
            std::fs::remove_file(output_file)?;
        }
        let mut inputs = [
            Track::new(&command.mastered)?,
            Track::new(&command.drum)?,
            Track::new(&command.bass)?,
            Track::new(&command.other)?,
            Track::new(&command.vocal)?,
        ];
        let mut nistem = NIStem::new_with_consistent_streams(output_file,ctx)?;
        if command.copy_id3tags_from_mastered {
            nistem.clone(&command.mastered)?;
        }
        let mut read = 0;
        let sample_rate: u64 = ctx.sample_rate.into();
        let pb = ProgressBar::new(2 * inputs[0].total() as u64);
            pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {percent}% ({eta})")
                .unwrap()
                .progress_chars("#>-"));

        loop {
            let mut data = vec![
                vec![0f32; 204800],
                vec![0f32; 204800],
                vec![0f32; 204800],
                vec![0f32; 204800],
                vec![0f32; 204800],
            ];

            let eof = {
                let size = 0;
                for (i, buf) in data.iter_mut().enumerate() {
                    let size = inputs[i].read(None, buf)?;
                    if i == 0 {
                        read += size;
                    }
                }
                size != data[0].len()
            };
            pb.set_position(read as u64 / sample_rate);
            nistem.write_consistent(data)?;
            if eof {
                break;
            }
        }

        pb.finish_with_message(format!("Processed {}", output_file.display()));
        nistem.flush(
            nistem::Atom {
                stems: [
                    nistem::AtomStem{
                        color: ctx.drum_stem_color.to_owned(),
                        name: ctx.drum_stem_label.to_owned(),
                    },
                    nistem::AtomStem{
                        color: ctx.bass_stem_color.to_owned(),
                        name: ctx.bass_stem_label.to_owned(),
                    },
                    nistem::AtomStem{
                        color: ctx.other_stem_color.to_owned(),
                        name: ctx.other_stem_label.to_owned(),
                    },
                    nistem::AtomStem{
                        color: ctx.vocal_stem_color.to_owned(),
                        name: ctx.vocal_stem_label.to_owned(),
                    },
                ],
                version: 1,
                ..Default::default()
            }
        )?;
    Ok(false)
}


#[cfg(test)]
mod tests {

    use stemgen::nistem::{Codec, SampleRate};

    use crate::{cli::CreateArgs, create::create, Cli, Commands};

    #[test]
    fn test_create_command() {
        let ctx = Cli {
            force: true,
            verbose: false,
            codec: Codec::FLAC,
            sample_rate: SampleRate::Hz48000,
            command: Commands::Create(CreateArgs {
                output:"../test_create_command.stem.mp4".into(),
                mastered:"../testdata/Oddchap - Sound 104.mp3".into(),
                drum:"../testdata/Oddchap - Sound 104.mp3".into(),
                bass:"../testdata/Oddchap - Sound 104.mp3".into(),
                other:"../testdata/Oddchap - Sound 104.mp3".into(),
                vocal:"../testdata/Oddchap - Sound 104.mp3".into(),
                copy_id3tags_from_mastered: true,
            }),
            ..Default::default()
        };
        if let Commands::Create(command) = &ctx.command {
            let result = create(&ctx, command);
            assert!(
                matches!(result, Ok(false)),
                "Expected value to match pattern, but got: {result:?}"
            );
        } else {
            unreachable!("unexpected command value")
        }
    }
}
