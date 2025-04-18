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
    print_version,
    print_supported_models,
)
from .demucs import Demucs
from .track import Track
from .nistemfile import NIStemFile


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
        help="Display verbose information which may be useful for debugging",
    )
    @click.option(
        "--use-alac/--use-aac",
        default=False,
        help="The codec to use for the stem stream stored in the output MP4.",
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
    @click.option(
        "--list-models",
        is_flag=True,
        callback=print_supported_models,
        help="List detected and supported models usable by demucs and exit",
        expose_value=False,
        is_eager=True,
    )
    @click.option(
        "--version",
        is_flag=True,
        callback=print_version,
        expose_value=False,
        is_eager=True,
        help="Display the stemgen version and exit",
    )
    @functools.wraps(func)
    def wrapper(*args, **kwargs):
        return func(*args, **kwargs)

    return wrapper


@click.group()
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
    use_alac,
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
    click.echo("Preparing...")
    demucs = Demucs(
        repo=repo,
        model=model,
        shifts=shifts,
        device=device,
        overlap=overlap,
        jobs=jobs,
    )
    has_failure = False
    for file in files:
        file = str(Path(file).resolve())
        filename = ".".join(os.path.basename(file).split(".")[:-1])
        dst = str(Path(f"{output}/{filename}.{ext}").resolve())

        if not force and os.path.exists(dst):
            click.secho(
                f"Cannot proceed with {os.path.basename(file)}: stem file already exist in output directory!",
                bold=True,
                fg="red",
                err=True,
            )
            has_failure |= True
            continue
        click.echo(f"Processing {filename}...")

        src = Track(file)
        samples = src.read()
        original, stems = None, []

        with click.progressbar(
            length=samples.shape[1] * len(demucs.weights),
            show_eta=True,
            show_percent=True,
            label="Demucsing",
        ) as progress:
            original, stems, warn = demucs.run(
                samples, update_cb=progress.update, finish_cb=progress.finish
            )
            if verbose:
                if len(warn) > 0:
                    click.secho(
                        f"\nThe following warnings were captured while demucs was processing:",
                        fg="yellow",
                    )

                for message in warn:
                    click.secho(
                        f"\t{warnings._formatwarnmsg_impl(message)}", fg="yellow"
                    )

        out = NIStemFile(dst, use_alac=use_alac)
        out.write(original, stems)
        out.update_metadata(
            file,
            stem_1_label=drum_stem_label,
            stem_1_color=drum_stem_color,
            stem_2_label=bass_stem_label,
            stem_2_color=bass_stem_color,
            stem_3_label=other_stem_label,
            stem_3_color=other_stem_color,
            stem_4_label=vocal_stem_label,
            stem_4_color=vocal_stem_color,
        )
        click.secho(f"Stem generated in {os.path.basename(dst)}", bold=True, fg="green")
    if has_failure:
        exit(1)


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
    use_alac,
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
    original = Track(mastered)
    stems = {
        "drums": Track(drum),
        "bass": Track(bass),
        "other": Track(other),
        "vocals": Track(vocal),
    }

    out = NIStemFile(output, use_alac=use_alac)
    out.write(original.read(), {k: v.read() for k, v in stems.items()})
    out.update_metadata(
        mastered if copy_id3tags_from_mastered else None,
        stem_1_label=drum_stem_label,
        stem_1_color=drum_stem_color,
        stem_2_label=bass_stem_label,
        stem_2_color=bass_stem_color,
        stem_3_label=other_stem_label,
        stem_3_color=other_stem_color,
        stem_4_label=vocal_stem_label,
        stem_4_color=vocal_stem_color,
    )
    click.secho(f"Stem create in {os.path.basename(output)}", bold=True, fg="green")


if __name__ == "__main__":
    main()
