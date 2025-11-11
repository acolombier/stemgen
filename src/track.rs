use std::{collections::HashMap, path::PathBuf};

use ffmpeg_next::{
    codec, decoder, format::{self, context}, frame::Audio, media, software::resampling, ChannelLayout, Packet, Rational
};
use taglib::AttachedPicture;

use crate::constant::{Metadata, MetadataValue};

const OUTPUT_SAMPLE_RATE: i32 = 44100;
const OUTPUT_CHANNELS: usize = 2;

pub struct Track {
    path: PathBuf,
    ctx: context::Input,
    index: usize,
    resampler: resampling::context::Context,
    decoder: decoder::Audio,
    overrun: Vec<f32>,
    eof_sent: bool,
}

impl Track {
    pub fn new(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let ctx = format::input(&path)?;

        let stream = ctx
            .streams()
            .best(media::Type::Audio)
            .ok_or("unable to find an audio stream")?;
        let index = stream.index();

        let context_decoder =
            ffmpeg_next::codec::context::Context::from_parameters(stream.parameters())?;
        let decoder = context_decoder.decoder().audio()?;

        // Use proper channel layout if decoder has empty layout
        let input_layout = if decoder.channel_layout().channels() == 2 && decoder.channel_layout().bits() == 0 {
            ChannelLayout::STEREO
        } else {
            decoder.channel_layout()
        };

        let resampler = resampling::context::Context::get(
            decoder.format(),
            input_layout,
            decoder.rate(),
            format::Sample::F32(format::sample::Type::Packed),
            ChannelLayout::STEREO,
            OUTPUT_SAMPLE_RATE as u32,
        )?;

        Ok(Self {
            path: path.clone(),
            ctx,
            index,
            resampler,
            decoder,
            overrun: Vec::new(),
            eof_sent: false,
        })
    }

    pub fn args(&self) -> (codec::Parameters, Rational) {
        let stream = self
            .ctx
            .streams()
            .find(|s| s.index() == self.index)
            .unwrap();
        (stream.parameters(), stream.time_base())
    }

    pub fn total(&self) -> i64 {
        let stream = self.ctx.stream(self.index).unwrap();
        stream.time_base().numerator() as i64 * stream.duration()
            / stream.time_base().denominator() as i64
    }
    pub fn total_samples(&self) -> i64 {
        let stream = self.ctx.stream(self.index).unwrap();
        // Calculate duration in seconds
        let duration_sec = stream.time_base().numerator() as i64 * stream.duration()
            / stream.time_base().denominator() as i64;
        // Multiply by output sample rate and channels for interleaved sample count
        duration_sec * OUTPUT_SAMPLE_RATE as i64 * OUTPUT_CHANNELS as i64
    }
}

impl Track {
    pub fn read(
        &mut self,
        mut original_packets: Option<&mut Vec<Packet>>,
        buf: &mut [f32],
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let mut write_pos = 0;

        if !self.overrun.is_empty() {
            let to_copy = std::cmp::min(self.overrun.len(), buf.len());
            buf[..to_copy].copy_from_slice(&self.overrun[..to_copy]);
            write_pos += to_copy;

            self.overrun.drain(..to_copy);

            if write_pos >= buf.len() {
                return Ok(write_pos);
            }
        }

        if self.eof_sent {
            return Ok(write_pos);
        }

        let mut packets = self.ctx.packets();

        while write_pos < buf.len() {
            // Read next packet
            let eof = if let Some((stream, packet)) = packets.next() {
                if stream.index() != self.index {
                    continue;
                }
                if let Some(ref mut original_packets) = original_packets {
                    original_packets.push(packet.clone());
                }

                self.decoder.send_packet(&packet)?;
                false
            } else {
                if !self.eof_sent {
                    self.decoder.send_eof()?;
                    self.eof_sent = true;
                }
                true
            };

            let mut decoded = Audio::empty();
            while self.decoder.receive_frame(&mut decoded).is_ok() {
                if decoded.channel_layout().channels() == 2 && decoded.channel_layout().bits() == 0 {
                    decoded.set_channel_layout(ChannelLayout::STEREO);
                }

                let mut resampled = Audio::empty();
                self.resampler.run(&decoded, &mut resampled)?;

                // For packed stereo: resampled.samples() returns frame count
                // The actual float count in the buffer is samples * channels
                let frame_count = resampled.samples();
                let channels = resampled.channel_layout().channels() as usize;
                let float_count = frame_count * channels;

                let output = resampled.plane::<f32>(0);

                // SAFETY: For packed format, the underlying buffer contains frame_count * channels floats,
                // but plane() returns a slice with wrong length. We reconstruct the correct slice.
                let output_correct = unsafe {
                    std::slice::from_raw_parts(output.as_ptr(), float_count)
                };

                let remaining_space = buf.len() - write_pos;

                if output_correct.len() <= remaining_space {
                    buf[write_pos..write_pos + output_correct.len()].copy_from_slice(output_correct);
                    write_pos += output_correct.len();
                } else {
                    buf[write_pos..].copy_from_slice(&output_correct[..remaining_space]);
                    write_pos = buf.len();

                    self.overrun.extend_from_slice(&output_correct[remaining_space..]);
                    return Ok(write_pos);
                }
            }

            if eof {
                // Flush resampler
                loop {
                    let mut resampled = Audio::empty();
                    match self.resampler.flush(&mut resampled) {
                        Ok(Some(_)) => {
                            // For packed stereo: same fix as above
                            let frame_count = resampled.samples();
                            let channels = resampled.channel_layout().channels() as usize;
                            let float_count = frame_count * channels;

                            let output = resampled.plane::<f32>(0);

                            let output_correct = unsafe {
                                std::slice::from_raw_parts(output.as_ptr(), float_count)
                            };

                            let remaining_space = buf.len() - write_pos;

                            if output_correct.len() <= remaining_space {
                                buf[write_pos..write_pos + output_correct.len()].copy_from_slice(output_correct);
                                write_pos += output_correct.len();
                            } else {
                                buf[write_pos..].copy_from_slice(&output_correct[..remaining_space]);
                                write_pos = buf.len();
                                self.overrun.extend_from_slice(&output_correct[remaining_space..]);
                                return Ok(write_pos);
                            }
                        }
                        _ => break,
                    }
                }
                break;
            }
        }

        Ok(write_pos)
    }
    pub fn tags(&self) -> HashMap<Metadata, MetadataValue> {
        taglib::File::new(&self.path)
            .map(|f| {
                f.tag()
                    .map(|tags| {
                        let mut metadata = HashMap::new();
                        if let Some(value) = tags.title() {
                            metadata.insert(Metadata::Title, value.into());
                        }
                        if let Some(value) = tags.artist() {
                            metadata.insert(Metadata::Artist, value.into());
                        }
                        if let Some(value) = tags.album() {
                            metadata.insert(Metadata::Release, value.into());
                        }
                        if let Some(value) = tags.comment() {
                            metadata.insert(Metadata::Label, value.into());
                        }
                        if let Some(value) = tags.genre() {
                            metadata.insert(Metadata::Genre, value.into());
                        }
                        if let Some(value) = tags.track() {
                            metadata.insert(Metadata::TrackNo, value.into());
                        }
                        metadata
                    })
                    .unwrap_or(HashMap::new())
            })
            .unwrap_or(HashMap::new())
    }
    pub fn covers(&self) -> Vec<AttachedPicture> {
        taglib::File::new(&self.path)
            .map(|f| f.pictures().unwrap_or(vec![]))
            .unwrap_or(vec![])
    }
}
