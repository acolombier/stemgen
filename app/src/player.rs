use std::{
    cmp::{self, Ordering},
    fs,
    io::{BufReader, SeekFrom, prelude::*},
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{self, AtomicUsize},
    },
    thread::{self, sleep},
    time::Duration,
};

use iced::{
    futures::{
        self, SinkExt, Stream, StreamExt,
        channel::mpsc::{self, Receiver, Sender},
        executor::block_on,
    },
    stream,
};
use rodio::{OutputStream, Sink, buffer::SamplesBuffer, source};

use crate::{
    app::{Message, Player, Server},
    model::{File, RenderedFile},
};

enum PlayData {
    Chunk([Vec<f32>; 4]),
    Info(usize),
}

#[derive(Debug)]
struct Play {
    file: RenderedFile,
    buffered: Arc<AtomicUsize>,
    current: usize,
    mask: u8,
}

enum Context {
    Idle,
    Stopping,
    Exit,
    Playing(Play),
    Seeking(Play, f32),
}

impl Context {
    pub fn seek(&mut self, pos: f32) {
        let r = std::mem::replace(self, Context::Idle);
        *self = match r {
            Context::Playing(play)| Context::Seeking(play, _) => Context::Seeking(play, pos),
            s => s,
        };

    }
    pub fn playing(&mut self, current: f32) {
        let r = std::mem::replace(self, Context::Idle);
        *self = match r {
            Context::Seeking(mut play, _) => {
                play.buffered.store(0, atomic::Ordering::Release);
                play.current = play.file.seek(current);
                Context::Playing(play)
            },
            s => s,
        };

    }
}

pub(crate) fn player_run() -> impl Stream<Item = Message> {
    stream::channel(2, |mut app| async move {
        let (sender, mut app_ch) = mpsc::channel(1);

        if let Err(e) = app.send(Message::Player(Player::Ready(sender))).await {
            println!("Unable to send the read message: {:?}", e);
        }

        let state = Arc::new((Mutex::new(Context::Idle), Condvar::new()));
        let context = state.clone();

        let reader_worker = thread::spawn(move || {
            let (_stream, handle) = OutputStream::try_default().unwrap();
            let output = Sink::try_new(&handle).unwrap();

            let mut sleep_till_next_tick = 0;

            'main: loop {
                if sleep_till_next_tick != 0 {
                    sleep(Duration::from_nanos(sleep_till_next_tick));
                    sleep_till_next_tick = 0;
                }
                let (context_lock, cvar) = &*context;
                let mut state = context_lock.lock().unwrap();
                while let Context::Idle = *state {
                    state = cvar.wait(state).unwrap();
                }

                match &mut *state {
                    Context::Idle => {}
                    Context::Seeking(playing, progress) => {
                        output.clear();
                        let total_sample = playing.file.total_samples().unwrap();
                        let mut state = context_lock.lock().unwrap();
                        state.playing(*progress);
                        output.play();
                    }
                    Context::Stopping => {
                        sleep_till_next_tick = 0;
                        println!("Stop playback");
                        *state = Context::Idle;
                    }
                    Context::Exit => break,
                    Context::Playing(playing) => {
                        // println!("playing context");
                        // TODO request track size to server
                        if playing.buffered.load(atomic::Ordering::Relaxed) > 4 {
                            sleep_till_next_tick =
                                (2048.0 / 44100.0 / 2.0 * 1_000_000_000.0) as u64;
                            // println!("Sleep for {} ns", sleep_till_next_tick);
                            continue 'main;
                        }
                        let offset = playing.current;
                        let total = playing.file.total_samples().unwrap() as usize;
                        // println!(
                        //     "filesize: {}, stem_sample_count: {}, offset: {}",
                        //     file.metadata().unwrap().len(),
                        //     stem_sample_count,
                        //     offset
                        // );
                        let buff_len = cmp::min(total - offset, 2048) as usize;

                        let mut buffers = vec![Some(vec![0.0f32; buff_len])];

                        buffers.extend((0..4).map(|idx| {
                            if playing.mask & (1u8 << idx) == 0 {
                                None
                            } else {
                                Some(vec![0f32; buff_len])
                            }
                        }));

                        let len = if let [main, drum, bass, other, vocals] = &mut buffers[..] {
                            playing.file.read(main.as_deref_mut(), drum.as_deref_mut(), bass.as_deref_mut(), other.as_deref_mut(), vocals.as_deref_mut()).unwrap()
                        } else {
                            unreachable!("a buffer was missing from the argument list")
                        };

                        let mut samples = buffers.remove(0).unwrap();
                        if !buffers.iter().all(|b| b.is_some()) {
                            samples.fill(0.0);
                            for buffer in buffers.iter_mut() {
                                if let Some(buffer) = buffer {
                                    for s in 0..buff_len {
                                        samples[s] += buffer[s];
                                    }
                                }
                            }
                        }

                        if !samples.is_empty() {
                            playing.buffered.fetch_add(1, atomic::Ordering::Release);
                            playing.current += len;

                            // println!("Playing sample: {}", playing.current);
                            output.append(source::Done::new(
                                SamplesBuffer::new(2, 44100, samples),
                                playing.buffered.clone(),
                            ));

                            if let Err(e) = block_on(app.send(Message::Player(Player::Progress(
                                offset as f32 / total as f32,
                            )))) {
                                println!("Unable to send the progress message: {:?}", e);
                            }
                        }
                        println!("{} {} {} {}", buff_len, len, playing.current, total);
                        if total <= playing.current {
                            println!("Finished reading {:?}", playing.file.id());
                            *state = Context::Idle;
                            continue 'main;
                        }
                    }
                }
            }
            output.sleep_until_end();
        });

        loop {
            futures::select! {
                command = app_ch.select_next_some() => {
                    match command {
                        Player::PlayFile(file_id, mask) => {
                            let file = RenderedFile::existing(file_id).unwrap();
                            println!("Playing: {:?} with {}", file_id, mask);
                            let (context_lock, condvar) = &*state;
                            let mut state = context_lock.lock().unwrap();
                            match &mut*state {
                                Context::Playing(Play { file: current_file, mask: current_mask, ..}) if current_file.id() == file_id => {
                                    *current_mask = mask;
                                }
                                _ => {
                                    *state = Context::Playing(Play { file, buffered: Arc::new(AtomicUsize::new(0)), current: 0 , mask: mask });
                                }
                            }
                            condvar.notify_one();
                        }
                        Player::Stop => {
                            let (context_lock, condvar) = &*state;
                            let mut state = context_lock.lock().unwrap();
                            *state = Context::Stopping;
                            condvar.notify_one();
                        }
                        Player::Seek(progress) => {
                            let (context_lock, _) = &*state;
                            let mut state = context_lock.lock().unwrap();
                            state.seek(progress);
                        }
                        _ =>{
                            println!("Unimpplemented command {:?}", command);
                        }
                    }
                }
                complete => break
            }
        }
        reader_worker.join().unwrap();
    })
}

#[cfg(test)]
mod tests {
    fn test_player(){
        // generate Rendered
        // emulate play
        // assert sample count is consistent
    }
}
