# Stemgen

![GitHub License](https://img.shields.io/github/license/acolombier/stemgen)
![GitHub Release](https://img.shields.io/github/v/release/acolombier/stemgen?include_prereleases)
![Docker Image](https://img.shields.io/docker/v/aclmb/stemgen)

> NOTE: Stemgen currently doesn't have a stable release. Please use carefully!

Stemgen is a library and tool that can be used to generate NI stem files
from most audio files. It is inspired from the the tool of the same name
[Stemgen](https://stemgen.dev). Here is how it compares:

- Transparent: no binaries or opaque processes required the generation.
  Everything is available open source, from the
  [demucsing](https://github.com/facebookresearch/demucs), till the STEM
  metadata generation.
- More flexible: support virtually any input format and codex and allows full
  customisation of the stem metadata.
- Dynamic: easy to ship or run on the go with Docker, or to script to
  generate many STEM at once.

Under the hood, it uses:

- Facebook's [demucs](https://github.com/facebookresearch/demucs) to split
  the signal into multiple audio stream
- Torch to generate the audio container with all the stream
- Some Taglib sources to generate the STEM metadata
- Taglib to manage the traditional audio metadata

## Install

> Currently, the tool was only tested on `linux/amd64`. All used dependency
> are meant to be cross platform, but some additional work my be required to
> get it working natively. Please
> [open a issue](https://github.com/acolombier/stemgen/issues) if your platform
> isn't supported

```sh
pip install -e "git+https://github.com/acolombier/stemgen.git@0.1.0#egg=stemgen"
```

### Ubuntu 22.04 / Debian Bookworm / PopOS 22.04

```sh
# Install FFmpeg and TagLib 2.0
sudo apt install -y ffmpeg cmake libutfcpp-dev
wget -O taglib.tar.gz https://github.com/taglib/taglib/releases/download/v2.0.1/taglib-2.0.1.tar.gz
tar xf taglib.tar.gz
cd taglib-2.0.1
cmake -DCMAKE_INSTALL_PREFIX=/usr \
  -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_SHARED_LIBS=ON .
make -j
sudo make install
cd ..
rm -rf taglib-2.0.1 taglib.tar.gz
```

## Usage

```text
Usage: stemgen [OPTIONS] FILES... OUTPUT

  Generate a NI STEM file out of an audio stereo file.

  FILES   path(s) to a file supported by the FFmpeg codec available on your
  machine

  OUTPUT  path to an existing directory where to store the generated STEM
  file(s)

Options:
  --model <model_name>            Demucs model.
  --device <cpu or cuda>          Device for the demucs model inference
  --ext TEXT                      Extension for the STEM file
  --force                         Proceed even if the output file already
                                  exists
  --verbose                       Display verbose information which may be
                                  useful for debugging
  --repo DIRECTORY                The local directory to use to fetch models
                                  for demucs.
  --model TEXT                    The model to use with demucs. Use --list-
                                  models to list the supported models. Default
                                  to htdemucs fine-trained
  --shifts INTEGER                Number of random shifts for equivariant
                                  stabilization to use for demucs. Increase
                                  separation time but improves quality for
                                  Demucs. 10 was used in the original paper.
  --overlap FLOAT                 Overlap between the splits to use for
                                  demucs.
  --jobs INTEGER                  The number of jobs to use for demucs.
  --use-alac / --use-aac          The codec to use for the stem stream stored
                                  in the output MP4.
  --drum-stem-label <label>       Custom label for the drum STEM (the first
                                  one)
  --drum-stem-color <hex-color>   Custom color for the drum STEM (the first
                                  one)
  --bass-stem-label <label>       Custom label for the drum STEM (the second
                                  one)
  --bass-stem-color <hex-color>   Custom color for the drum STEM (the second
                                  one)
  --other-stem-label <label>      Custom label for the drum STEM (the third
                                  one)
  --other-stem-color <hex-color>  Custom color for the drum STEM (the third
                                  one)
  --vocal-stem-label <label>      Custom label for the drum STEM (the fourth
                                  and last one)
  --vocal-stem-color <hex-color>  Custom color for the drum STEM (the fourth
                                  and last one)
  --list-models                   List detected and supported models usable by
                                  demucs and exit
  --version                       Display the stemgen version and exit
  --help                          Show this message and exit.

```

### Example

- Simple usage

  ```sh
  stemgen "Artist - Title.mp3" .
  ```

- Using `htdemucs_ft` for better result, but more memory usage (see
  [the benchmark section](#memory-benchmark))

  ```sh
  stemgen "Artist - Title.mp3" . --model htdemucs_ft
  ```

### Note on STEM customisation

NI recommends using the following labels for the stem:

- Acid
- Atmos
- Bass
- Bassline
- Chords
- Clap
- Comp
- Donk
- Drone
- Drums
- FX
- Guitar
- HiHat
- Hits
- Hook
- Kick
- Lead
- Loop
- Melody
- Noise
- Pads
- Reece
- SFX
- Snare
- Stabs
- SubBass
- Synths
- Toms
- Tops
- Vocals
- Voices

## Memory Benchmark

Benchmarks are performed with a **3m30s** song with CUDA, running on the
following machine spec:

```plain
12th Gen Intel(R) Core(TM) i7-12700H
64 GB RAM
NVIDIA GeForce RTX 3050
Samsung 980 PRO SSD
```

| Model                | Memory usage peak | Real time |
|----------------------|-------------------|-----------|
| `htdemucs` (default) |            1.8 GB | 1m6.427s  |
| `htdemucs_ft`        |            3.3 GB | 32.637s   |

## Docker image

If you don't want to install `stemgen` on your machine, you can use the Docker
container. Here the simple way to use it:

```sh
docker run \
    -v /path/to/folder:/path/to/folder \
    -it --rm \
    aclmb/stemgen:0.1.0 \
        /path/to/folder/Artist\ -\ Title.mp3 \
        /path/to/folder
```

if you want to use CUDA acceleration, and cache the model not to download it
every time, you can do the following:

```sh
docker run \
    -v /path/to/folder:/path/to/folder \
    -v stemgen_torch_cache:/root/.cache/torch/hub/ \
    -it --gpus --rm \
    aclmb/stemgen:0.1.0 \
        /path/to/folder/Artist\ -\ Title.mp3 \
        /path/to/folder
```

## License

Stemgen is released under a [MIT license](LICENSE). `stembox`, which is a
component of Stemgen used to generate stem manifest is released under a
[LGPL License](stembox/LICENSE) as it reuse battle-tested code from TagLib
