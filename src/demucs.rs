use std::{error::Error, fmt::Write as _, hash::{BuildHasher, Hasher, RandomState}, io::{Read, Write as _}, path::{Path, PathBuf}};
use ort::{session::builder::SessionBuilder, value::{Shape, TensorElementType, ValueType}};
use ndarray::{s, ArrayViewMut, ShapeBuilder};
use ort::{session::{builder::GraphOptimizationLevel, Session}, value::Tensor};

#[cfg(feature = "cuda")]
use ort::{execution_providers::CUDAExecutionProvider};

use ureq::{
    config::Config,
    tls::{RootCerts, TlsConfig, TlsProvider}
};

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
    pub threads: usize,
}

impl Default for DemusOpts {
    fn default() -> Self {
        Self { threads: 2, device: Device::CPU }
    }
}

struct DummyDownloader {}
impl DownloadProgress for DummyDownloader {
    fn start(&self, _: usize) {}

    fn progress(&self, _: usize) {}

    fn complete(&self) {}
}

impl Demucs {
    pub fn new_from_file(model: &Model, ops: DemusOpts) -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_from_file_with_downloader(model, ops, DummyDownloader{})
    }

    pub fn new_from_file_with_downloader(model: &Model, ops: DemusOpts, downloader: impl DownloadProgress) -> Result<Self, Box<dyn std::error::Error>> {
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
            .commit();

        let mut session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(ops.threads)?;

        let session = match model {
            Model::Local(path) => session.commit_from_file(path)?,
            Model::Url(url) => model_from_url(session, url, downloader)?,
        };

        if session.inputs().len() != 1 {
            return Err("expected model to have one input".into())
        }

        if session.outputs().len() != 1 {
            return Err("expected model to have one output".into())
        }

        let input_name = {
            let input = session.inputs().first().unwrap();
            match &input.dtype() {
                ValueType::Tensor {
                    ty: TensorElementType::Float32,
                    shape,
                    ..
                    // TODO support multiple buffer length and channel
                } if *shape == Shape::new([1, 2, 343980]) => {
                    Ok(input.name().to_owned())
                }
                _ => {
                    Err(format!("unsupported input format: {}", input.dtype()))
                }
            }
        }?;

        let output_name = {
            let output = session.outputs().first().unwrap();
            match &output.dtype() {
                ValueType::Tensor {
                    ty: TensorElementType::Float32,
                    shape,
                    ..
                    // TODO support multiple buffer length and channel
                } if *shape == Shape::new([1, 4, 2, 343980]) => {
                    Ok(output.name().to_owned())
                }
                _ => {
                    Err(format!("unsupported output format: {}", output.dtype()))
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

    pub fn into_session(self) -> Session {
        self.session
    }

}


#[cfg(target_os = "linux")]
#[must_use]
fn cache_dir_default() -> Option<PathBuf> {
    use crate::constant::STEMGEN_ROOT;

	std::env::var_os("XDG_CACHE_HOME")
		.and_then(|p|{
            let path = PathBuf::from(p);
		    if path.is_absolute() { Some(path) } else { None }
        })
		.or_else(|| std::env::home_dir().map(|h| h.join(".cache").join(STEMGEN_ROOT)))
}

#[cfg(target_os = "macos")]
#[must_use]
fn cache_dir_default() -> Option<PathBuf> {
    use crate::constant::STEMGEN_ROOT;
	std::env::home_dir().map(|h| h.join("Library/Caches").join(STEMGEN_ROOT))
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn cache_dir_default() -> Option<PathBuf> {
	None
}

pub fn cache_dir() -> Option<PathBuf> {
	std::env::var_os("STEMGEN_CACHE_DIR").map(PathBuf::from).or_else(cache_dir_default)
}

pub trait DownloadProgress {
    fn start(&self, total: usize);
    fn progress(&self, current: usize);
    fn complete(&self);
}

pub fn random_identifier() -> String {
	let mut state = RandomState::new().build_hasher().finish();
	std::iter::repeat_with(move || {
		state ^= state << 13;
		state ^= state >> 7;
		state ^= state << 17;
		state
	})
	.take(12)
	.map(|i| b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"[i as usize % 62] as char)
	.collect()
}

fn model_from_url(mut session: SessionBuilder, url: &str, downloader: impl DownloadProgress) -> Result<Session, Box<dyn Error>> {
	let mut download_dir = cache_dir()
		.expect("could not determine cache directory")
		.join("models");
	if std::fs::create_dir_all(&download_dir).is_err() {
		download_dir = std::env::current_dir().expect("Failed to obtain current working directory");
	}

	let model_filename = <sha2::Sha256 as sha2::Digest>::digest(url).into_iter().fold(String::new(), |mut s, b| {
		let _ = write!(&mut s, "{:02x}", b);
		s
	});
	let model_filepath = download_dir.join(&model_filename);
	if model_filepath.exists() {
        session.commit_from_file(model_filepath).map_err(|e|e.into())
	} else {
		let agent = Config::builder()
			.tls_config(
				TlsConfig::builder()
					.root_certs(RootCerts::WebPki)
					.provider(TlsProvider::Rustls)
					.build()
			)
			.build()
			.new_agent();

		let resp = agent.get(url).call().map_err(|e| format!("Error downloading to file: {e}"))?;

		let len = resp
			.headers()
			.get("Content-Length")
			.and_then(|h| h.to_str().ok())
			.and_then(|s| s.parse::<usize>().ok())
			.expect("Missing Content-Length header");
		downloader.start(len);

		let mut reader = resp.into_body().into_with_config().limit(u64::MAX).reader();
		let temp_filepath = download_dir.join(format!("tmp_{}.{model_filename}", random_identifier()));

		let f = std::fs::File::create(&temp_filepath).expect("Failed to create model file");
		let mut writer = std::io::BufWriter::new(f);

        let mut buf = vec![0u8; 16*1024];
        let mut bytes_io_count = 0;
        loop {
            let read = reader.read(&mut buf)? as usize;
            writer.write_all(&buf[..read])?;
            bytes_io_count += read;
            downloader.progress(bytes_io_count);

            if read == 0{
                break
            }
        }
        downloader.complete();

		if bytes_io_count != len {
			return Err(format!("Failed to download entire model; file only has {bytes_io_count} bytes, expected {len}").into());
		}

		drop(writer);

		match std::fs::rename(&temp_filepath, &model_filepath) {
			Ok(()) => session.commit_from_file(model_filepath).map_err(|e|e.into()),
			Err(e) => {
				if model_filepath.exists() {
					let _ = std::fs::remove_file(temp_filepath);
                    session.commit_from_file(model_filepath).map_err(|e|e.into())
				} else {
					Err(format!("Failed to download model: {e}").into())
				}
			}
		}
    }
}
