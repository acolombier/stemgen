pub mod audio_ops;
pub mod constant;
pub mod demucs;
pub mod nistem;
pub mod track;

#[cfg(test)]
mod tests {
    use std::io::Write;

    use ffmpeg_next::codec;

    use crate::{
        demucs::{Demucs, DemusOpts, Model},
        nistem::{Atom, NIStem},
        track::Track,
    };

    #[test]
    fn test_complete_pipeline() {
        let demucs = Demucs::new_from_file(&Model::default(), DemusOpts::default());
        assert!(
            demucs.is_ok(),
            "Expected value to match pattern, but got: {demucs:?}"
        );
        let mut demucs = demucs.unwrap();

        let input = Track::new(&"./testdata/Oddchap - Sound 104.mp3".into());
        assert!(
            input.is_ok(),
            "Expected value to match pattern, but got: {:?}",
            input.err().unwrap()
        );
        let mut input = input.unwrap();

        let mut buf = vec![0f32; 444866];
        let mut original_packets = vec![];
        let data = loop {
            match input.read(Some(&mut original_packets), &mut buf) {
                Ok(size) => {
                    if size != buf.len() {
                        panic!("reach unexpected state")
                    }
                }
                Err(err) => panic!("{}", err),
            }
            match demucs.send(&buf) {
                Ok(Some(data)) => break data,
                Err(err) => panic!("{}", err),
                _ => {}
            }
            match demucs.flush() {
                Ok(data) => break data,
                Err(err) => panic!("{}", err),
            }
        };

        let mut f = std::fs::File::create("data.pcm").unwrap();
        for sample in buf.iter() {
            f.write_all(&f32::to_le_bytes(*sample)).unwrap();
        }

        for stem in &data {
            assert_eq!(stem.len(), 444866);
            for (idx, sample) in stem.iter().enumerate() {
                assert!(!sample.is_infinite(), "found infinite at {idx}");
                assert!(!sample.is_nan(), "found nan at {idx}");
            }
        }
        let output_filename = std::env::temp_dir().join("test_complete_pipeline.stem.mp4".to_string());
        if output_filename.exists() {
            std::fs::remove_file(&output_filename).unwrap();
        }
        let nistem = NIStem::new_with_preserved_original(
            &output_filename,
            input.args(),
            (codec::Id::AAC, 44100),
        );
        assert!(
            nistem.is_ok(),
            "Expected value to match pattern, but got: {:?}",
            nistem.err().unwrap()
        );
        let mut nistem = nistem.unwrap();
        let result = nistem.write_preserved(original_packets, data);
        assert!(
            result.is_ok(),
            "Expected value to match pattern, but got: {:?}",
            result.err().unwrap()
        );
        let result = nistem.flush(Atom::default());
        assert!(
            result.is_ok(),
            "Expected value to match pattern, but got: {:?}",
            result.err().unwrap()
        );

        std::fs::remove_file(&output_filename).unwrap();
    }
}
