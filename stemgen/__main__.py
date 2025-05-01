import click

from torch import cuda
import click
import os
from pathlib import Path
import functools

from .cli import (
    validate_device,
    validate_ext,
    validate_model,
    validate_stem_label,
    validate_stem_color,
    validate_sample_rate_for_codec,
    print_version,
    print_supported_models,
    enable_verbose_ffmpeg_log_level,
)
from . import app
from .demucs import Demucs
from .track import Track
from .nistemfile import NIStemFile
from .constant import Codec, SampleRate


def common_options(func):
    @click.option(
        "--force",
        default=False,
        is_flag=True,
        help="Proceed even if the output file already exists",
    )
    @click.option(
        "--verbose",
        default=False,
        is_flag=True,
        callback=enable_verbose_ffmpeg_log_level,
        help="Display verbose information which may be useful for debugging",
    )
    @click.option(
        "--codec",
        default=Codec.AAC,
        callback=validate_sample_rate_for_codec,
        help="The codec to use for the stem stream stored in the output MP4.",
        type=click.Choice(Codec, case_sensitive=False),
    )
    @click.option(
        "--sample-rate",
        default=str(SampleRate.Hz44100),
        callback=validate_sample_rate_for_codec,
        help="The sample rate to use for the output.",
        type=click.Choice([str(s.value) for s in SampleRate]),
    )
    @click.option(
        "--drum-stem-label",
        callback=validate_stem_label,
        metavar="<label>",
        help="Custom label for the drum stem (the first one)",
    )
    @click.option(
        "--drum-stem-color",
        callback=validate_stem_color,
        metavar="<hex-color>",
        help="Custom color for the drum stem (the first one)",
    )
    @click.option(
        "--bass-stem-label",
        callback=validate_stem_label,
        metavar="<label>",
        help="Custom label for the bass stem (the second one)",
    )
    @click.option(
        "--bass-stem-color",
        callback=validate_stem_color,
        metavar="<hex-color>",
        help="Custom color for the bass stem (the second one)",
    )
    @click.option(
        "--other-stem-label",
        callback=validate_stem_label,
        metavar="<label>",
        help="Custom label for the other stem (the third one)",
    )
    @click.option(
        "--other-stem-color",
        callback=validate_stem_color,
        metavar="<hex-color>",
        help="Custom color for the other stem (the third one)",
    )
    @click.option(
        "--vocal-stem-label",
        callback=validate_stem_label,
        metavar="<label>",
        help="Custom label for the vocal stem (the fourth and last one)",
    )
    @click.option(
        "--vocal-stem-color",
        callback=validate_stem_color,
        metavar="<hex-color>",
        help="Custom color for the vocal stem (the fourth and last one)",
    )
    @functools.wraps(func)
    def wrapper(*args, **kwargs):
        return func(*args, **kwargs)

    return wrapper


@click.group()
@click.option(
    "--version",
    is_flag=True,
    callback=print_version,
    expose_value=False,
    is_eager=True,
    help="Display the stemgen version and exit",
)
@click.pass_context
def main(ctx, **kwargs):
    pass


@main.command()
@click.argument(
    "files",
    nargs=-1,
    required=True,
    type=click.Path(exists=True, dir_okay=False, readable=True),
)
@click.argument(
    "output",
    nargs=1,
    envvar="STEMGEN_OUTPUT",
    type=click.Path(exists=True, file_okay=False, dir_okay=True, writable=True),
)
@click.option(
    "--model",
    default="htdemucs",
    help="Demucs model.",
    metavar="<model_name>",
)
@click.option(
    "--device",
    default="cuda" if cuda.is_available() else "cpu",
    metavar="<cpu or cuda>",
    callback=validate_device,
    help="Device for the demucs model inference",
)
@click.option(
    "--ext",
    default="stem.mp4",
    callback=validate_ext,
    help="Extension for the STEM file",
)
@click.option(
    "--repo",
    default=None,
    expose_value=True,
    help="The local directory to use to fetch models for demucs.",
    is_eager=True,
    type=click.Path(exists=True, file_okay=False, dir_okay=True, readable=True),
)
@click.option(
    "--model",
    default="htdemucs",
    callback=validate_model,
    help="The model to use with demucs. Use --list-models to list the supported models. Default to htdemucs fine-trained",
)
@click.option(
    "--shifts",
    default=1,
    help="Number of random shifts for equivariant stabilization to use for demucs. Increase separation time but improves quality for Demucs. 10 was used "
    "in the original paper.",
)
@click.option(
    "--overlap",
    default=0.25,
    help="Overlap between the splits to use for demucs.",
)
@click.option(
    "--jobs",
    default=1,
    help="The number of jobs to use for demucs.",
)
@click.option(
    "--list-models",
    is_flag=True,
    callback=print_supported_models,
    help="List detected and supported models usable by demucs and exit",
    expose_value=False,
    is_eager=True,
)
@common_options
def generate(
    files,
    output,
    device,
    force,
    verbose,
    ext,
    repo,
    model,
    shifts,
    overlap,
    jobs,
    codec,
    sample_rate,
    drum_stem_label,
    drum_stem_color,
    bass_stem_label,
    bass_stem_color,
    other_stem_label,
    other_stem_color,
    vocal_stem_label,
    vocal_stem_color,
):
    """Generate a NI STEM file out of an audio stereo file.

    FILES   path(s) to a file supported by the FFmpeg codec available on your machine

    OUTPUT  path to an existing directory where to store the generated STEM file(s)
    """
    app.generate(
        files,
        output,
        device,
        force,
        verbose,
        ext,
        repo,
        model,
        shifts,
        overlap,
        jobs,
        codec,
        sample_rate,
        drum_stem_label,
        drum_stem_color,
        bass_stem_label,
        bass_stem_color,
        other_stem_label,
        other_stem_color,
        vocal_stem_label,
        vocal_stem_color,
    )


@main.command()
@click.argument(
    "output",
    nargs=1,
    envvar="STEMGEN_OUTPUT",
    type=click.Path(file_okay=True, dir_okay=False, writable=True),
)
@click.option(
    "--mastered",
    required=True,
    type=click.Path(exists=True, dir_okay=False, readable=True),
    help="Source file for the pre-mastered track",
)
@click.option(
    "--drum",
    required=True,
    type=click.Path(exists=True, dir_okay=False, readable=True),
    help="Source file for the drum stem (the first one)",
)
@click.option(
    "--bass",
    required=True,
    type=click.Path(exists=True, dir_okay=False, readable=True),
    help="Source file for the bass stem (the second one)",
)
@click.option(
    "--other",
    required=True,
    type=click.Path(exists=True, dir_okay=False, readable=True),
    help="Source file for the other stem (the third one)",
)
@click.option(
    "--vocal",
    required=True,
    type=click.Path(exists=True, dir_okay=False, readable=True),
    help="Source file for the vocal stem (the fourth and last one)",
)
@click.option(
    "--copy-id3tags-from-mastered",
    is_flag=True,
    help="Copy all ID3 tags from the mastered track",
)
@common_options
def create(
    output,
    mastered,
    drum,
    bass,
    other,
    vocal,
    force,
    verbose,
    codec,
    sample_rate,
    drum_stem_label,
    drum_stem_color,
    bass_stem_label,
    bass_stem_color,
    other_stem_label,
    other_stem_color,
    vocal_stem_label,
    vocal_stem_color,
    copy_id3tags_from_mastered,
):
    """Create a NI STEM file out of existing stem tracks.

    OUTPUT  path to the generated STEM file
    """
    app.create(
        output,
        mastered,
        drum,
        bass,
        other,
        vocal,
        force,
        verbose,
        codec,
        sample_rate,
        drum_stem_label,
        drum_stem_color,
        bass_stem_label,
        bass_stem_color,
        other_stem_label,
        other_stem_color,
        vocal_stem_label,
        vocal_stem_color,
        copy_id3tags_from_mastered,
    )


if __name__ == "__main__":
    main()
