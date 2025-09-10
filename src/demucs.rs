use std::path::{Path, PathBuf};
use ort::tensor::{Shape, TensorElementType};
use ort::value::ValueType;
use ndarray::{s, ArrayViewMut, ShapeBuilder};
use ort::{session::{builder::GraphOptimizationLevel, Session}, value::Tensor};

#[cfg(feature = "cuda")]
use ort::{execution_providers::CUDAExecutionProvider};

use crate::constant::DEFAULT_MODEL;

#[derive(Debug)]
pub struct Demucs {
    session: Session,
    input_name: String,
    output_name: String,
    input_buffer: Vec<f32>,
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
    CUDA
}

impl std::fmt::Display for Device {
     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "cuda")]
            Device::CUDA => write!(f, "cuda"),
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
    pub threads: usize
}

impl Default for DemusOpts {
    fn default() -> Self {
        Self { threads: 2, device: Device::CPU }
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

        Ok(Self {
            session,
            input_name,
            output_name,
            input_buffer: Vec::with_capacity(2 * 343980),
        })

    }

    fn process(&mut self) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
        let tensor = Tensor::<f32>::from_array(ArrayViewMut::from_shape((1, 2, 343980).strides((343980 * 2, 1, 2)), &mut self.input_buffer)?.to_owned())?;
        let result = self.session.run(ort::inputs! {
            &self.input_name => tensor
        })?;
        let output = result[self.output_name.as_str()].try_extract_array::<f32>()?;
        let mut stems = vec![Vec::new(); 4];
        for (i, stem) in stems.iter_mut().enumerate() { // Iterate over the 4 items
            let mut offset = stem.len();
            stem.resize_with(offset+2 * 343980, ||0.0f32);

            let l_slice = output.slice(s![0, i, 0, ..]); // All L values for item i
            let r_slice = output.slice(s![0, i, 1, ..]); // All R values for item i

            for (l, r) in l_slice.iter().zip(r_slice.iter()) {
                stem[offset] = *l;
                stem[offset + 1] = *r;
                offset += 2;
            }
        }
        if self.input_buffer.len() == 2 * 343980 {
            self.input_buffer.clear();
        } else {
            let leftover = self.input_buffer.len() - 2 * 343980;
            let (left, right) = self.input_buffer.split_at_mut(2 * 343980);
            left[..leftover].copy_from_slice(right);
            self.input_buffer.resize(leftover, 0.0);
        }
        Ok(stems)
    }

    pub fn send(&mut self, sample_buffer: &[f32]) -> Result<Option<Vec<Vec<f32>>>, Box<dyn std::error::Error>> {
        if sample_buffer.len() % 2 != 0 {
            return Err("uneven number of sample".into());
        }

        self.input_buffer.extend_from_slice(sample_buffer);

        if self.input_buffer.len() >= 2 * 343980 {
            Ok(Some(self.process()?))
        } else {
            Ok(None)
        }
    }

    pub fn flush(&mut self) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
        let buffer_size = self.input_buffer.len();
        self.input_buffer.resize(2 * 343980, 0.0);
        let mut data = self.process()?;
        for stem in data.iter_mut() {
            stem.resize(buffer_size, 0.0f32);
        }
        Ok(data)
    }

}
