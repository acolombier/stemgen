import click
import re
from demucs.api import list_models
from pathlib import Path
from torchaudio.utils import ffmpeg_utils

from .constant import MAX_STEM_LABEL_LENGTH, AvLog, Codec, SampleRate
from . import __version__


def validate_device(ctx, param, value):
    if value.lower() not in ["cpu", "cuda"]:
        raise click.BadParameter("device must be 'cpu' or 'cuda'")
    return value.lower()


def validate_ext(ctx, param, value):
    if not value.lower().endswith(".mp4") and not value.lower().endswith(".m4a"):
        raise click.BadParameter("extension must be suffixed with '.mp4' or '.m4a'")
    return value


def validate_model(ctx, param, value):
    supported_models = sum(
        [
            list(l.keys())
            for l in list(
                list_models(
                    Path(ctx.params["repo"]) if ctx.params.get("repo") else None
                ).values()
            )
        ],
        [],
    )
    if value not in supported_models:
        raise click.BadParameter(
            "model not found in the repo. Use --list-models to list available models or --repo to use another repo"
        )
    return value


def validate_stem_label(ctx, param, value):
    if value and len(value) > MAX_STEM_LABEL_LENGTH:
        raise click.BadParameter(
            f"the stem label can only be {MAX_STEM_LABEL_LENGTH} char at max"
        )
    return value


def validate_stem_color(ctx, param, value):
    if value and not re.match("^#[0-9a-f]{6}$", value, flags=re.IGNORECASE):
        raise click.BadParameter(
            f"the stem color must be in hex-rgb format (e.g #AABBCC)"
        )
    return value


def print_version(ctx, param, value):
    if not value or ctx.resilient_parsing:
        return
    click.echo(__version__)
    ctx.exit()


def print_supported_models(ctx, param, value):
    if not value or ctx.resilient_parsing:
        return
    models = list_models(Path(ctx.params["repo"]) if ctx.params.get("repo") else None)
    click.echo("Bag of models:")
    click.echo("\n    ".join(models["bag"]))
    click.echo("Single models:")
    click.echo("\n    ".join(models["single"]))
    ctx.exit()


def enable_verbose_ffmpeg_log_level(ctx, param, value):
    if value:
        ffmpeg_utils.set_log_level(AvLog.VERBOSE)


# The opus codec only supports an output sample_rate of 48kHz
def validate_sample_rate_for_codec(ctx, param, value):
    if param.name == "sample_rate":
        value = SampleRate(int(value))

    def _do_validate(codec: Codec, sample_rate: SampleRate):
        if codec == Codec.OPUS and sample_rate != SampleRate.Hz48000:
            raise click.BadParameter(
                "Opus requires a sample rate of 48kHz. Set it using --sample-rate 48000"
            )

    # Validate after we have both params, independent of given order
    if param.name == "codec" and "sample_rate" in ctx.params:
        _do_validate(value, ctx.params["sample_rate"])
    elif param.name == "sample_rate" and "codec" in ctx.params:
        _do_validate(ctx.params["codec"], value)

    return value
