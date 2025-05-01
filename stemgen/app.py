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
    """Generate a NI STEM file out of an audio stereo file."""
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

        src = Track(file, demucs.sample_rate)
        samples = src.read()
        original, stems = None, []

        with click.progressbar(
            length=demucs.length(samples) * shifts,
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

        out = NIStemFile(dst, codec, demucs.sample_rate, sample_rate)
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
    """Create a NI STEM file out of existing stem tracks."""
    original = Track(mastered, sample_rate)
    stems = {
        "drums": Track(drum, sample_rate),
        "bass": Track(bass, sample_rate),
        "other": Track(other, sample_rate),
        "vocals": Track(vocal, sample_rate),
    }

    out = NIStemFile(output, codec, sample_rate, sample_rate)
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
