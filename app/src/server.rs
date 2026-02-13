use std::thread;

use iced::futures::{self, channel::mpsc, Stream};
use iced::futures::{SinkExt, StreamExt};
use iced::stream;
use itertools::Itertools;
use stemgen::demucs::{Demucs, DemusOpts, Model};
use stemgen::track::Track;
use tokio::runtime;

use crate::model::RenderedFile;
use crate::waveform;
use crate::{
    app::{Message, Server},
};
// struct Demucs {
//     buffer: Box<[f32]>
// }

// impl Demucs {
//     fn new() -> Self {
//         Self {
//             buffer: Box::new()
//         }
//     }
// }

pub fn server_loop() -> impl Stream<Item = Message> {
    stream::channel(5, |mut output| async move {
        thread::Builder::new().stack_size(64*1024*1024).spawn(move || {
            // Create the runtime
            let rt = runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                // });

                // // Spawn the root task
                // rt.block_on(async {
                let (app_command, mut app_ch) = mpsc::channel(5);

                let init_result = async {
                    output.send(Message::Server(Server::Ready(app_command))).await?;
                    let demucs = Demucs::new_from_file(&Model::default(), DemusOpts::default())?;
                    Ok::<_, Box<dyn std::error::Error>>(demucs)
                };

                init_result.await.unwrap();

                // TODO Handle init error
                println!("Ready to select!");
                loop {
                    futures::select! {
                        message = app_ch.select_next_some() => {
                            let result = async {
                                match message {
                                    // Server::LoadFiles(paths) => {
                                    //     let request = tonic::Request::new(FileRequest {
                                    //         paths: paths.iter().map(|p|p.to_str().unwrap().into()).collect(),
                                    //     });
                                    //     let sender = &mut output;
                                    //     // rt.block_on(async move {
                                    //         let mut stream = client.prepare_file(request).await.unwrap().into_inner();

                                    //         while let Some(file) = stream.next().await {
                                    //             let file = File::from_proto(file.unwrap());
                                    //             sender.send(Message::FileUpdated(file)).await.unwrap();
                                    //         }
                                    //     // });
                                    // }
                                    Server::SplitFile(file_id, path) => {
                                        println!("SplitFile: {:?}", path);
                                        let mut demucs = Demucs::new_from_file(&Model::default(), DemusOpts::default())?;

                                        let mut input = Track::new(&path)?;
                                        let mut read = 0;
                                        let mut rendered = RenderedFile::new(file_id).unwrap();
                                        // FIXME what is total returning? Document!
                                        let total_size = input.total() as usize;
                                        let sample_per_slice = total_size / waveform::SAMPLE_COUNT;
                                        let mut current_slice = 0;

                                        let mut waveforms = vec![vec![(0.0, 0.0); waveform::SAMPLE_COUNT]; 4];
                                        let mut waveform_overrun = vec![vec![0f32; sample_per_slice]; 4];

                                        loop {
                                            let mut buf: Vec<f32> = vec![0f32; 343980 * 2];
                                            let mut original_packets = Vec::with_capacity(512);

                                            let (data, eof) = loop {
                                                let size = input.read(Some(&mut original_packets), &mut buf)?;
                                                read += size;
                                                if let Some(mut data) = demucs.send(&buf[..size])? {
                                                    data.insert(0, buf[..size].to_vec());
                                                    break (data, false);
                                                }
                                                if size != buf.len() {
                                                    let mut data = demucs.flush()?;
                                                    data.insert(0, buf[..size].to_vec());
                                                    break (data, true);
                                                }
                                            };

                                            for (i, waveform) in waveforms.iter_mut().enumerate() {
                                                let mut slice = current_slice;
                                                // TODO handle current waveform overrun
                                                for samples in data[1 + i].chunks(sample_per_slice) {
                                                    if samples.len() != sample_per_slice {
                                                        waveform_overrun[i].resize(samples.len(), 0.0);
                                                        waveform_overrun[i].copy_from_slice(&samples);
                                                    }
                                                    // Using Max - TODO RMS
                                                    waveform[slice] = samples.into_iter()
                                                        .tuples::<(_, _)>()
                                                        .fold((0.0 as f32, 0.0 as f32), |(lacc, racc), (l, r)| {
                                                            (
                                                                if lacc > l.abs() { lacc } else { l.abs() },
                                                                if racc > r.abs() { racc } else { r.abs() },
                                                            )
                                                        });
                                                    slice+=1;
                                                }
                                            }
                                            current_slice += data[0].len() / sample_per_slice;
                                            rendered.write(original_packets, data).unwrap();

                                            output.send(Message::FileSplitProgress(file_id, (read as f32) / total_size as f32)).await?;

                                            if eof {
                                                rendered.complete()?;
                                                break;
                                            }
                                        }

                                        output.send(Message::FileSplitCompleted(file_id, rendered, waveforms)).await?;
                                        Ok::<_, Box<dyn std::error::Error>>(())
                                    }
                                    _ => {
                                        println!("Unsupported command: {:?}", message);
                                        Ok(())
                                    }
                                }
                            };
                            if let Err(err) = result.await {
                                println!("something when wrong: {}", err)
                            }
                        }
                        complete => {
                            break;
                        }
                    }
                }
            });
        }).unwrap();
    })
}
