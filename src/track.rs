use std::path::PathBuf;

use ffmpeg_next::{
    codec, decoder, format::{self, context}, frame::Audio, media, software::resampling, Packet, Rational
};

pub struct Track {
    ctx: context::Input,
    index: usize,
    resampler: resampling::context::Context,
    decoder: decoder::Audio,
    overrun: [f32; 10240],
    overrun_len: usize,
}

impl Track {
    pub fn new(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let ctx = format::input(&path)?;

        // format::context::input::dump(&ctx, 0, Some(path.to_str().ok_or("unable to read path")?));
        let stream = ctx
            .streams()
            .best(media::Type::Audio)
            .ok_or("unable to find an audio stream")
            ?;
        let index = stream.index();

        let context_decoder =
            ffmpeg_next::codec::context::Context::from_parameters(stream.parameters())?;
        let decoder = context_decoder.decoder().audio()?;

        let resampler = ffmpeg_next::software::resampling::context::Context::get(
            decoder.format(),
            decoder.channel_layout(),
            decoder.rate(),
            format::Sample::F32(format::sample::Type::Packed),
            ffmpeg_next::ChannelLayout::STEREO,
            44100,
        )
        ?;

        Ok(Self {
            ctx,
            index,
            resampler,
            decoder,
            overrun: [0f32; 10240],
            overrun_len: Default::default(),
        })
    }

    pub fn args(&self) -> (codec::Parameters, Rational) {
        let stream = self.ctx
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
}

impl Track {
    pub fn read(
        &mut self,
        mut original_packets: Option<&mut Vec<Packet>>,
        buf: &mut [f32],
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let mut read = 0;

        if self.overrun_len > 0 && self.overrun_len <= buf.len() {
            buf[..self.overrun_len].copy_from_slice(&self.overrun[..self.overrun_len]);
            read = self.overrun_len;
            self.overrun_len = 0;
        } else if self.overrun_len > buf.len() {
            buf.copy_from_slice(&self.overrun[..buf.len()]);
            self.overrun_len -= buf.len();
            return Ok(buf.len());
        }
        let mut packets = self.ctx.packets();

        let mut process = |mut resampled: Audio, buf: &mut [f32], read: usize| {
            let output = resampled.plane_mut(0);

            if output.len() > buf.len() - read {
                let (left, right) = output.split_at_mut(buf.len() - read);
                buf[read..].copy_from_slice(left);
                self.overrun[..right.len()].copy_from_slice(right);
                self.overrun_len = right.len();
                return buf.len() - read;
            }

            buf[read..read + output.len()].copy_from_slice(output);
            output.len()
        };

        while read < buf.len() {
            let eof = if let Some((stream, packet)) = packets.next() {
                if stream.index() != self.index {
                    continue;
                }
                original_packets = if let Some(original_packets) = original_packets {
                    original_packets.push(packet.clone());
                    Some(original_packets)
                } else {
                    None
                };
                // println!("packet {:?}", packet.pts());

                self.decoder.send_packet(&packet)?;
                false
            } else {
                self.decoder.send_eof()?;
                true
            };

            let mut decoded = Audio::empty();
            while self.decoder.receive_frame(&mut decoded).is_ok() {
                let mut resampled = Audio::empty();
                self.resampler.run(&decoded, &mut resampled)?;
                resampled.set_samples(resampled.samples() * decoded.planes()); // FIXME seems to be a bug upstream?
                // println!("frame {:?}", resampled.pts());
                read += process(resampled, buf, read);
            }
            if eof {
                let mut finished = false;
                while !finished {
                    let mut resampled = Audio::new(
                        self.resampler.output().format,
                        1024,
                        self.resampler.output().channel_layout,
                    );

                    finished = match self.resampler.flush(&mut resampled) {
                        Ok(None) => true,
                        Ok(_) | Err(_) => {
                            false
                        }
                    };
                    if resampled.planes() == 0 {
                        break;
                    }
                    resampled.set_samples(resampled.samples() * decoded.planes()); // FIXME seems to be a bug upstream?
                    read += process(resampled, buf, read);
                }
                break;
            }
        }
        Ok(read)
    }
}
