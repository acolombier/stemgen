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
- More flexible: support virtually any input format and codec and allows full
  customisation of the stem metadata.
- Dynamic: easy to ship or run on the go with Docker, or to script to
  generate many STEM at once.
- Producer friendly: offer a multi platform, open source altrernative to the
  NI's SteamCreator and allow stem creation from STEM tracks.

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

### Python package

Before you can install the Python package, you will need to install the
dependencies.

#### Ubuntu / PopOS

> [!WARNING]
> stemgen depends on pytorch-audio, which depends of FFmpeg 6. The default
version shipped on Ubuntu 24.10 and Debian Trixie and beyond is FFmpeg 7,
which is incompatible. Make sure to use a backport.

```sh
# Install FFmpeg, Boost and TagLib 2.0
sudo apt install -y ffmpeg libboost-python1.74-dev libboost-python1.74.0 cmake libutfcpp-dev
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

#### Fedora 40 and derivative

> [!WARNING]
> stemgen depends on pytorch-audio, which depends of FFmpeg 6. The default
version shipped on Fedora 41 and beyond is FFmpeg 7,
which is incompatible. Make sure to use a backport.

```sh
# Install FFmpeg, Boost and TagLib 2.0
sudo dnf install ffmpeg boost-python3 python3-pip g++ boost-devel \
  python3-devel cmake utf8cpp-devel
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

### PIP

Once all dependencies have been successfully installed, you can install the
python package with pip.

#### CPU (no GPU acceleration)

```sh
pip install "git+https://github.com/acolombier/stemgen.git@0.4.0#egg=stemgen" \
  --index-url "https://download.pytorch.org/whl/cpu" \
  --extra-index-url https://pypi.org/simple
```

#### CUDA (Nvidia acceleration)

> [!NOTE]
> You can use `cu118` instead of `cu124` for CUDA 11. (Older hardware/driver)

```sh
pip install "git+https://github.com/acolombier/stemgen.git@0.4.0#egg=stemgen" \
  --index-url "https://download.pytorch.org/whl/cu124" \
  --extra-index-url https://pypi.org/simple
```

#### Global

> [!WARNING]
> This will install PyTorch with all dependencies for any backends, inducing
> gigabytes of dependencies to download and store.

```sh
pip install "git+https://github.com/acolombier/stemgen.git@0.4.0#egg=stemgen"
```

### Docker (recommended)

If you don't want to install `stemgen` on your machine, you can use the Docker
container.

### Flavour

> [!WARNING]
> The main tag (`aclmb/stemgen:0.4.0`) will include PyTorch with all dependencies
> for any backends, inducing gigabytes of dependencies!

- CPU (no hardware acceleration): `aclmb/stemgen:0.4.0-cpu`
- Cuda 12 (Nvidia card): `aclmb/stemgen:0.4.0-cuda`
- Cuda 11 (older Nvidia card/driver): `aclmb/stemgen:0.4.0-cuda11`

Here the simple way to use it:

```sh
docker run \
    -v /path/to/folder:/path/to/folder \
    -it --rm \
    aclmb/stemgen:0.4.0-<Flavour> generate \
        /path/to/folder/Artist\ -\ Title.mp3 \
        /path/to/folder
```

if you want to use CUDA acceleration (only relevant for the `generate`
command), and cache the model not to download it every time, you can do the
following:

```sh
docker run \
    -v /path/to/folder:/path/to/folder \
    -v stemgen_torch_cache:/root/.cache/torch/hub/ \
    -it --gpus --rm \
    aclmb/stemgen:0.4.0-<Flavour> generate \
        /path/to/folder/Artist\ -\ Title.mp3 \
        /path/to/folder
```

## Usage

```text
stemgen generate [GENERATE OPTIONS, COMMON OPTIONS] FILES... OUTPUT

  Generate a NI STEM file out of an audio stereo file.

  FILES   path(s) to a file supported by the FFmpeg codec available on your
  machine

  OUTPUT  path to an existing directory where to store the generated STEM
  file(s)

stemgen create [GENERATE OPTIONS, COMMON OPTIONS] OUTPUT

  Create a NI STEM file out of existing stem tracks.

  OUTPUT  path to the generated STEM file

Options for "genetate":
  --model <model_name>            Demucs model.
  --device <cpu or cuda>          Device for the demucs model inference
  --ext TEXT                      Extension for the STEM file
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
  --list-models                   List detected and supported models usable by
                                  demucs and exit

Options for "create":
  --mastered FILE                 Source file for the pre-mastered track
                                  [required]
  --drum FILE                     Source file for the drum stem (the first
                                  one)  [required]
  --bass FILE                     Source file for the bass stem (the second
                                  one)  [required]
  --other FILE                    Source file for the other stem (the third
                                  one)  [required]
  --vocal FILE                    Source file for the vocal stem (the fourth
                                  and last one)  [required]
  --copy-id3tags-from-mastered    Copy all ID3 tags from the mastered track

Common options:
  --force                         Proceed even if the output file already
                                  exists
  --verbose                       Display verbose information which may be
                                  useful for debugging
  --use-alac / --use-aac          The codec to use for the stem stream stored
                                  in the output MP4.
  --drum-stem-label <label>       Custom label for the drum stem (the first
                                  one)
  --drum-stem-color <hex-color>   Custom color for the drum stem (the first
                                  one)
  --bass-stem-label <label>       Custom label for the bass stem (the second
                                  one)
  --bass-stem-color <hex-color>   Custom color for the bass stem (the second
                                  one)
  --other-stem-label <label>      Custom label for the other stem (the third
                                  one)
  --other-stem-color <hex-color>  Custom color for the other stem (the third
                                  one)
  --vocal-stem-label <label>      Custom label for the vocal stem (the fourth
                                  and last one)
  --vocal-stem-color <hex-color>  Custom color for the vocal stem (the fourth
                                  and last one)
  --version                       Display the stemgen version and exit
  --help                          Show this message and exit.

```

### Example

#### Generating a STEM track from a Stereo MP3

- Simple usage

  ```sh
  stemgen generate "Artist - Title.mp3" .
  ```

- Using `htdemucs_ft` for better result, but more memory usage (see
  [the benchmark section](#memory-benchmark))

  ```sh
  stemgen generate "Artist - Title.mp3" . --model htdemucs_ft
  ```

#### Create a STEM track from pre-splitted STEM tracks

- Simple usage

  ```sh
  stemgen create
    --mastered "Pre-mastered mix.mp3" \
    --drum "drum part.mp3" \
    --bass "bass part.mp3" \
    --other "other part.mp3" \
    --vocal "vocal part.mp3" \
    "Artist - Title.stem.mp4"
  ```

- Customize the STEM metadata

  ```sh
  stemgen create \
    --mastered "Pre-mastered mix.mp3" \
    --drum "Kick part.mp3" \
    --bass "SubBass part.mp3" \
    --other "synth part.mp3" \
    --vocal "Voices part.mp3" \
    --drum-stem-label "Kick" \
    --drum-stem-color "#37e4d0" \
    --bass-stem-label "SubBass" \
    --bass-stem-color "#656bba" \
    --other-stem-label "Synths" \
    --other-stem-color "#52d034" \
    --vocal-stem-label "Voices" \
    --vocal-stem-color "#daae2a" \
    "Artist - Title.stem.mp4"
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
| `htdemucs` (default) |            1.8 GB | 32.637s   |
| `htdemucs_ft`        |            3.3 GB | 1m6.427s  |

## License

Stemgen is released under a [MIT license](LICENSE). `stembox`, which is a
component of Stemgen used to generate stem manifest is released under a
[LGPL License](stembox/LICENSE) as it reuse battle-tested code from TagLib
