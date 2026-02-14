use std::path::{Path, PathBuf};
use ort::tensor::{Shape, TensorElementType};
use ort::value::ValueType;
use ndarray::{s, ArrayViewMut, ShapeBuilder};
use ort::{session::{builder::GraphOptimizationLevel, Session}, value::Tensor};

use crate::audio_ops::{weighted_accumulate, accumulate_weights, normalize_by_weights};

#[cfg(feature = "cuda")]
use ort::{execution_providers::CUDAExecutionProvider};

#[cfg(feature = "coreml")]
use ort::execution_providers::{CoreMLExecutionProvider, coreml::{CoreMLComputeUnits}};

use crate::constant::DEFAULT_MODEL;

const SEGMENT_SAMPLES: usize = 343980;
const CHANNELS: usize = 2;
const NUM_STEMS: usize = 4;

#[derive(Debug)]
pub struct Demucs {
    session: Session,
    input_name: String,
    output_name: String,
    input_buffer: Vec<f32>,
    overlap: f32,
    transition_power: f32,
    segment_output_buffer: Vec<[Vec<f32>; CHANNELS]>,
}

#[derive(Debug, Clone)]
pub enum Model {
    Local(PathBuf),
    Url(String)
}

impl Default for Model {
    fn default() -> Self {
        Model::Url(DEFAULT_MODEL.to_owned())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Device {
    #[default]
    CPU,
    #[cfg(feature = "cuda")]
    CUDA,
    #[cfg(feature = "coreml")]
    CoreML
}

impl std::fmt::Display for Device {
     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "cuda")]
            Device::CUDA => write!(f, "cuda"),
            #[cfg(feature = "coreml")]
            Device::CoreML => write!(f, "coreml"),
            Device::CPU => write!(f, "cpu"),
        }
    }
}

impl TryFrom<&str> for Device {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            #[cfg(feature = "cuda")]
            "cuda" => Ok(Device::CUDA),
            #[cfg(feature = "coreml")]
            "coreml" => Ok(Device::CoreML),
            "cpu" => Ok(Device::CPU),
            _ => Err("unsupported device".to_owned()),
        }
    }
}

impl std::fmt::Display for Model {
     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match  self {
            Model::Local(path_buf) => write!(f, "{}", path_buf.to_str().unwrap()),
            Model::Url(url) => write!(f, "{url}"),
        }
    }
}

impl TryFrom<&str> for Model {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.starts_with("http") {
            Ok(Self::Url(value.to_owned()))
        } else {
            let path = Path::new(&value);
            if !path.exists() {
                Err("unable to find the model".to_owned())
            } else {
                Ok(Self::Local(path.to_path_buf()))
            }
        }
    }
}

pub struct DemusOpts {
    pub device: Device,
    pub threads: usize,
    pub overlap: f32,
    pub transition_power: f32,
}

impl Default for DemusOpts {
    fn default() -> Self {
        Self {
            threads: 2,
            device: Device::CPU,
            overlap: 0.25,
            transition_power: 1.0,
        }
    }
}

impl Demucs {
    pub fn new_from_file(model: &Model, ops: DemusOpts) -> Result<Self, Box<dyn std::error::Error>> {
        ort::init()
            .with_execution_providers(
            match ops.device {
                #[cfg(feature = "cuda")]
                Device::CUDA => vec![
                    CUDAExecutionProvider::default()
                        .with_tf32(true)
                        // TODO support specific device passing?
                        .with_device_id(0)
                        // FIXME seem to wrongly set the memory limit to 0?
                        // .with_memory_limit(1 * 1024 * 1024 * 1024)
                        .build()
                        .error_on_failure()
                ],
                #[cfg(feature = "coreml")]
                Device::CoreML => vec![
                    CoreMLExecutionProvider::default()
                        // FIXME: There is currently a huge memory leak with CoreML runtime in ort crate
                        .with_compute_units(CoreMLComputeUnits::CPUAndGPU)  // Use GPU for hardware acceleration
                        .build()
                        .error_on_failure()
                ],
                Device::CPU => vec![]
            })
            .commit()?;

        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(ops.threads)?;

        let session = match model {
            Model::Local(path) => session.commit_from_file(path)?,
            Model::Url(url) => session.commit_from_url(url)?,
        };

        if session.inputs.len() != 1 {
            return Err("expected model to have one input".into())
        }

        if session.outputs.len() != 1 {
            return Err("expected model to have one output".into())
        }

        let input_name = {
            let input = session.inputs.first().unwrap();
            match &input.input_type {
                ValueType::Tensor {
                    ty: TensorElementType::Float32,
                    shape,
                    ..
                    // TODO support multiple buffer length and channel
                } if *shape == Shape::new([1, 2, 343980]) => {
                    Ok(input.name.to_owned())
                }
                _ => {
                    Err(format!("unsupported input format: {}", input.input_type))
                }
            }
        }?;

        let output_name = {
            let output = session.outputs.first().unwrap();
            match &output.output_type {
                ValueType::Tensor {
                    ty: TensorElementType::Float32,
                    shape,
                    ..
                    // TODO support multiple buffer length and channel
                } if *shape == Shape::new([1, 4, 2, 343980]) => {
                    Ok(output.name.to_owned())
                }
                _ => {
                    Err(format!("unsupported output format: {}", output.output_type))
                }
            }
        }?;

        // Pre-allocate segment output buffer
        let segment_output_buffer: Vec<[Vec<f32>; CHANNELS]> = (0..NUM_STEMS)
            .map(|_| [
                Vec::with_capacity(SEGMENT_SAMPLES),
                Vec::with_capacity(SEGMENT_SAMPLES)
            ])
            .collect();

        Ok(Self {
            session,
            input_name,
            output_name,
            input_buffer: Vec::with_capacity(CHANNELS * SEGMENT_SAMPLES),
            overlap: ops.overlap,
            transition_power: ops.transition_power,
            segment_output_buffer,
        })

    }

    /// Process a single segment through the model
    /// Reuses internal buffer to avoid allocations
    fn process_segment(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let tensor = Tensor::<f32>::from_array(
            ArrayViewMut::from_shape(
                (1, CHANNELS, SEGMENT_SAMPLES).strides((SEGMENT_SAMPLES * CHANNELS, 1, CHANNELS)),
                &mut self.input_buffer
            )?.to_owned()
        )?;
        let result = self.session.run(ort::inputs! {
            &self.input_name => tensor
        })?;
        let output = result[self.output_name.as_str()].try_extract_array::<f32>()?;

        // Reuse buffer - clear and refill instead of allocating
        for i in 0..NUM_STEMS {
            let l_slice = output.slice(s![0, i, 0, ..]);
            let r_slice = output.slice(s![0, i, 1, ..]);

            self.segment_output_buffer[i][0].clear();
            self.segment_output_buffer[i][1].clear();
            self.segment_output_buffer[i][0].extend(l_slice.iter().copied());
            self.segment_output_buffer[i][1].extend(r_slice.iter().copied());
        }

        Ok(())
    }

    /// Create triangular weight vector for windowing
    fn create_weight_vector(segment_samples: usize, transition_power: f32) -> Vec<f32> {
        let mut weight = vec![0.0f32; segment_samples];
        let half_segment = segment_samples / 2;

        // First half: linear ramp up
        for i in 0..half_segment {
            weight[i] = (i + 1) as f32 / half_segment as f32;
        }

        // Second half: linear ramp down
        for i in half_segment..segment_samples {
            weight[i] = (segment_samples - i) as f32 / half_segment as f32;
        }

        // Apply transition power
        if transition_power != 1.0 {
            for w in weight.iter_mut() {
                *w = w.powf(transition_power);
            }
        }

        weight
    }

    /// Process entire audio with optional overlap for better quality
    /// When overlap=0, processes segments sequentially without windowing (fast)
    /// When overlap>0, uses overlapping windows with blending (better quality)
    /// Returns planar format: Vec<[left_channel, right_channel]>
    pub fn process<F>(&mut self, audio: &[f32], mut progress_callback: F) -> Result<Vec<[Vec<f32>; CHANNELS]>, Box<dyn std::error::Error>>
    where
        F: FnMut(usize, usize),
    {
        if audio.len() % CHANNELS != 0 {
            return Err("audio length must be multiple of channel count".into());
        }

        let length = audio.len() / CHANNELS;

        // Fast path: no overlap
        if self.overlap == 0.0 {
            // Pre-allocate output buffers with exact size (no reallocation needed)
            let mut output: Vec<[Vec<f32>; CHANNELS]> = (0..NUM_STEMS)
                .map(|_| [
                    Vec::with_capacity(length),
                    Vec::with_capacity(length)
                ])
                .collect();

            let mut segment_offset = 0;

            while segment_offset < length {
                let chunk_length = std::cmp::min(SEGMENT_SAMPLES, length - segment_offset);

                // Prepare input buffer - zero pad if needed
                self.input_buffer.clear();
                self.input_buffer.resize(CHANNELS * SEGMENT_SAMPLES, 0.0);

                // Copy audio chunk (audio is interleaved)
                let sample_count = chunk_length * CHANNELS;
                let src_offset = segment_offset * CHANNELS;
                self.input_buffer[..sample_count].copy_from_slice(&audio[src_offset..src_offset + sample_count]);

                // Process segment - fills segment_output_buffer
                self.process_segment()?;

                // Copy from reused buffer to output (trim to actual chunk length)
                for stem_idx in 0..NUM_STEMS {
                    output[stem_idx][0].extend_from_slice(&self.segment_output_buffer[stem_idx][0][..chunk_length]);
                    output[stem_idx][1].extend_from_slice(&self.segment_output_buffer[stem_idx][1][..chunk_length]);
                }

                segment_offset += SEGMENT_SAMPLES;
                progress_callback(segment_offset, length);
            }

            return Ok(output);
        }

        // Quality path: with overlap and windowing
        let stride_samples = ((1.0 - self.overlap) * SEGMENT_SAMPLES as f32) as usize;
        let weight = Self::create_weight_vector(SEGMENT_SAMPLES, self.transition_power);

        // Pre-allocate output buffers with exact size
        let mut output: Vec<[Vec<f32>; CHANNELS]> = (0..NUM_STEMS)
            .map(|_| [vec![0.0f32; length], vec![0.0f32; length]])
            .collect();
        let mut sum_weight = vec![0.0f32; length];

        let mut segment_offset = 0;
        while segment_offset < length {
            let chunk_length = std::cmp::min(SEGMENT_SAMPLES, length - segment_offset);

            // Prepare input buffer
            self.input_buffer.clear();
            self.input_buffer.resize(CHANNELS * SEGMENT_SAMPLES, 0.0);

            // Copy audio chunk (audio is interleaved)
            let sample_count = chunk_length * CHANNELS;
            let src_offset = segment_offset * CHANNELS;
            self.input_buffer[..sample_count].copy_from_slice(&audio[src_offset..src_offset + sample_count]);

            // Process segment - fills segment_output_buffer
            self.process_segment()?;

            // Apply weights and accumulate (planar format - better cache locality)
            for stem_idx in 0..NUM_STEMS {
                // Left channel
                weighted_accumulate(
                    &self.segment_output_buffer[stem_idx][0][..chunk_length],
                    &weight[..chunk_length],
                    &mut output[stem_idx][0][segment_offset..segment_offset + chunk_length]
                );
                // Right channel
                weighted_accumulate(
                    &self.segment_output_buffer[stem_idx][1][..chunk_length],
                    &weight[..chunk_length],
                    &mut output[stem_idx][1][segment_offset..segment_offset + chunk_length]
                );
            }

            // Accumulate weights
            accumulate_weights(
                &weight[..chunk_length],
                &mut sum_weight[segment_offset..segment_offset + chunk_length]
            );

            segment_offset += stride_samples;
            progress_callback(segment_offset.min(length), length);
        }

        // Normalize by sum of weights (planar format)
        for stem_idx in 0..NUM_STEMS {
            normalize_by_weights(&mut output[stem_idx][0], &sum_weight);
            normalize_by_weights(&mut output[stem_idx][1], &sum_weight);
        }

        Ok(output)
    }

}
