#[cfg(feature = "benchmark")]
use chrono::Utc;
use ndarray::{ArrayViewMut, ShapeBuilder, s};
use ort::{
    execution_providers::CUDAExecutionProvider,
    // memory::{AllocationDevice, AllocatorType, MemoryType}
};
use ort::{
    execution_providers::get_gpu_device,
    // memory:: MemoryInfo
};
use ort::{
    session::{Session, builder::GraphOptimizationLevel},
    value::Tensor,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("GPU: {:?}", get_gpu_device());

    ort::init()
        .with_execution_providers(vec![
            CUDAExecutionProvider::default()
                .with_tf32(true)
                .with_device_id(0)
                // .with_memory_limit(1 * 1024 * 1024 * 1024)
                .build()
                .error_on_failure(),
        ])
        .commit()?;
    let session = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?;

    println!("session ready!");

    // let memory = MemoryInfo::new(AllocationDevice::CUDA_PINNED, 0, AllocatorType::Device, MemoryType::Default)?;
    let mut session = session.commit_from_url("https://github.com/acolombier/demucs/releases/download/v4.0.1-18-g1640988-onnxmodel/htdemucs.onnx")?;

    let mut input = vec![0.0f32; 343980 * 2];

    // Attempt 1 - Cudarc (Note I had to up[date the example to match cudarc 0.17, needed for CUDA 12.9])
    for _ in 0..10 {
        println!("about to allocate");
        #[cfg(feature = "benchmark")]
        let mut start_time = Utc::now().time();
        let tensor = Tensor::<f32>::from_array(
            ArrayViewMut::from_shape((1, 2, 343980).strides((343980 * 2, 1, 2)), &mut input)?
                .to_owned(),
        )?;

        #[cfg(feature = "benchmark")]
        {
            let diff = Utc::now().time() - start_time;
            println!(
                "about to start inference. Total time taken to run is {} ms",
                diff.num_milliseconds()
            );
            start_time = Utc::now().time();
        }
        #[cfg(not(feature = "benchmark"))]
        println!("about to start inference");
        let result = session.run(ort::inputs! {
            "input" => tensor
        })?;
        #[cfg(feature = "benchmark")]
        {
            let diff = Utc::now().time() - start_time;
            println!(
                "about to fetch output. Total time taken to run is {} ms",
                diff.num_milliseconds()
            );
            start_time = Utc::now().time();
        }
        #[cfg(not(feature = "benchmark"))]
        println!("about to fetch output");
        let output = result["output"].try_extract_array::<f32>()?;
        #[cfg(feature = "benchmark")]
        {
            let diff = Utc::now().time() - start_time;
            println!(
                "finished. Total time taken to run is {} ms",
                diff.num_milliseconds()
            );
            start_time = Utc::now().time();
        }
        #[cfg(not(feature = "benchmark"))]
        println!("finished");

        // // Attempt 2 - With binding
        // let mut binding = session.create_binding()?;
        // for _ in 0..10 {
        //     println!("about to allocate");
        //     let start_time = Utc::now().time();
        //     let tensor: ort::value::Value<ort::value::TensorValueType<f32>> = Tensor::<f32>::from_array(ArrayViewMut::from_shape((1, 2, 343980).strides((343980 * 2, 1, 2)), &mut input)?.to_owned())?;
        //     binding.bind_input("input", &tensor)?;
        //     binding.bind_output_to_device("output", &memory)?;
        //     let end_time = Utc::now().time();
        //     let diff = end_time - start_time;
        //     println!("about to start inference. Total time taken to run is {} ms", diff.num_milliseconds());
        //     let start_time = Utc::now().time();
        //     let mut result = session.run_binding(&binding)?;
        //     let end_time = Utc::now().time();
        //     let diff = end_time - start_time;
        //     println!("about to fetch output. Total time taken to run is {} ms", diff.num_milliseconds());
        //     let start_time = Utc::now().time();
        //     let output = result.remove("output").ok_or("cannot extract data")?;
        //     let output = output.try_extract_array::<f32>()?;
        //     let end_time = Utc::now().time();
        //     let diff = end_time - start_time;
        //     println!("finished. Total time taken to run is {} ms", diff.num_milliseconds());
        //     let start_time = Utc::now().time();

        let mut stems = vec![Vec::new(); 4];
        for (i, stem) in stems.iter_mut().enumerate() {
            // Iterate over the 4 items
            let mut offset = stem.len();
            stem.resize_with(offset + 2 * 343980, || 0.0f32);

            let l_slice = output.slice(s![0, i, 0, ..]); // All L values for item i
            let r_slice = output.slice(s![0, i, 1, ..]); // All R values for item i

            for (l, r) in l_slice.iter().zip(r_slice.iter()) {
                stem[offset] = *l;
                stem[offset + 1] = *r;
                offset += 2;
            }
        }
        println!("{:?}", &stems[0][100..130]);

        #[cfg(feature = "benchmark")]
        {
            let diff = Utc::now().time() - start_time;
            println!("Total time taken to run is {} ms", diff.num_milliseconds());
        }
    }

    Ok(())
}
