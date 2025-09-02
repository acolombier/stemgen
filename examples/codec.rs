use ffmpeg_next::{
    ChannelLayout, Packet,
    codec::{self, Compliance, Id},
    encoder, format,
    frame::Audio,
    software::resampling,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        return Err(format!("Usage: {} <OutputFile> <CODEC> <SampleRate>", args[0]).into());
    }
    let mut ctx = format::output(&args[1]).unwrap();
    unsafe {
        // Needed for OPUS and FLAC?
        (*ctx.as_mut_ptr()).strict_std_compliance = -2;
        //     (*ctx.as_mut_ptr()).flags |= AVFMT_FLAG_GENPTS;
    }

    let codec = match args[2].as_str() {
        "AAC" => Id::AAC,
        "ALAC" => Id::ALAC,
        "FLAC" => Id::FLAC,
        "OPUS" => Id::OPUS,
        "MP3" => Id::MP3,
        _ => return Err("Unsupported format".into()),
    };

    ffmpeg_next::init().unwrap();
    ffmpeg_next::log::set_level(ffmpeg_next::log::Level::Trace);

    let sample_rate = args[3].parse::<i32>()
        .map_err(|_| "Invalid sample rate")
        .unwrap();
    let bit_rate = None;
    let rc_max_rate = None;
    let mut overrun: Vec<Vec<f32>> = vec![Default::default(); 4];

    let codec = encoder::find(codec)
        .ok_or(ffmpeg_next::Error::InvalidData)
        .unwrap();
    let mut idx_encoders = Vec::new();
    let mut formats = codec
        .audio()
        .unwrap()
        .formats()
        .ok_or(ffmpeg_next::Error::InvalidData)
        .unwrap();
    let format = formats
        .next()
        .ok_or(ffmpeg_next::Error::InvalidData)
        .unwrap();

    println!("codec: {:?}", codec.name());

    for _ in 0..1 {
        let mut encoder = codec::context::Context::new().encoder().audio().unwrap();
        encoder.compliance(Compliance::Experimental);
        encoder.set_flags(codec::flag::Flags::GLOBAL_HEADER);
        encoder.set_rate(sample_rate);
        encoder.set_channel_layout(ChannelLayout::STEREO);
        encoder.set_format(format);
        if let Some(bit_rate) = bit_rate {
            encoder.set_bit_rate(bit_rate);
        }
        if let Some(rc_max_rate) = rc_max_rate {
            encoder.set_max_bit_rate(rc_max_rate);
        }

        unsafe {
            (*encoder.as_mut_ptr()).frame_size = 1024;
        }
        let mut ost = ctx.add_stream(codec).unwrap();
        let encoder = encoder.open_as(codec).unwrap();
        ost.set_parameters(&encoder);
        println!(
            "codec: {:?} {:?}",
            ost.parameters().medium(),
            ost.parameters().id()
        );
        let resampler = resampling::Context::get(
            format::Sample::F32(format::sample::Type::Packed),
            ffmpeg_next::ChannelLayout::STEREO,
            sample_rate as u32,
            encoder.format(),
            encoder.channel_layout(),
            encoder.rate(),
        )
        .unwrap();
        idx_encoders.push((ost.index(), encoder, resampler, 0));
    }

    ctx.write_header().unwrap();

    let mut buf = vec![0.0f32; sample_rate as usize * 10 * 2];
    let freq = 220f32;
    for i in 0..buf.len() / 2 {
        buf[2 * i] = f32::cos(freq * i as f32 * std::f32::consts::PI / sample_rate as f32) * 0.15;
        buf[2 * i + 1] =
            f32::cos(freq * i as f32 * std::f32::consts::PI / sample_rate as f32) * 0.15;
    }
    let buf = vec![buf];

    for (stream_idx, ((idx, encoder, resampler, timestamp), mut frames)) in idx_encoders.iter_mut().zip(buf).enumerate() {
        let frame_size = 2 * encoder.frame_size() as usize;
        if !overrun[stream_idx].is_empty() {
            frames = {
                let mut v = overrun[stream_idx].clone();
                v.extend(frames);
                v
            };
            overrun[stream_idx].clear();
        }
        if frames.len() % frame_size != 0 {
            let new_len = frames.len() - frames.len() % frame_size;
            overrun[stream_idx].extend_from_slice(&frames[new_len..]);
            frames.resize(frames.len() - frames.len() % frame_size, 0.0);
        }
        for chunk in frames.chunks(frame_size) {
            let mut frame = Audio::new(
                format::Sample::F32(format::sample::Type::Packed),
                chunk.len(),
                ffmpeg_next::ChannelLayout::STEREO,
            );
            frame.set_rate(sample_rate as u32);
            frame.set_pts(Some(*timestamp as i64));
            *timestamp += chunk.len();
            frame.plane_mut(0).copy_from_slice(chunk);
            frame.set_samples(chunk.len() / 2);
            let mut resampled = Audio::empty();
            resampler.run(&frame, &mut resampled).unwrap();
            encoder.send_frame(&resampled).unwrap();
            let mut encoded: Packet = Packet::empty();
            while encoder.receive_packet(&mut encoded).is_ok() {
                encoded.set_stream(*idx);
                encoded.write(&mut ctx).unwrap();
            }
        }
    }

    for (stream_idx, (idx, encoder, resampler, timestamp)) in idx_encoders.iter_mut().enumerate() {
        if !overrun[stream_idx].is_empty() {
            let chunk = &overrun[stream_idx];
            let mut frame = Audio::new(
                format::Sample::F32(format::sample::Type::Packed),
                chunk.len(),
                ffmpeg_next::ChannelLayout::STEREO,
            );
            frame.set_rate(sample_rate as u32);
            frame.set_pts(Some(*timestamp as i64));
            *timestamp += chunk.len();
            frame.plane_mut(0).copy_from_slice(chunk);
            frame.set_samples(chunk.len() / 2);
            let mut resampled = Audio::empty();
            resampler.run(&frame, &mut resampled).unwrap();
            encoder.send_frame(&resampled).unwrap();
            overrun[stream_idx].clear();
        }
        if resampler.delay().is_some() {
            let mut resampled = Audio::empty();
            resampler.flush(&mut resampled).unwrap();
            encoder.send_frame(&resampled).unwrap();
        }
        encoder.send_eof().unwrap();
        let mut encoded = Packet::empty();
        while encoder.receive_packet(&mut encoded).is_ok() {
            encoded.set_stream(*idx);
            encoded.write(&mut ctx).unwrap();
        }
    }
    ctx.write_trailer().unwrap();

    Ok(())
}
