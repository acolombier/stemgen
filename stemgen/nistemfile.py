import click

import tagpy
import tagpy.id3v2
import tagpy.ogg.flac
import tagpy.mp4
import logging
from torchaudio.io import StreamWriter, CodecConfig
import stembox
import torch

from .constant import CHUNK_SIZE, Codec

logger = logging.getLogger(__file__)

SUPPORTED_TAGS = [
    "title",
    "artist",
    "album",
    "comment",
    "genre",
    "year",
    "track",
]


def _extract_cover(file_ref: tagpy.FileRef) -> tagpy.mp4.CoverArt:
    tag = file_ref.tag()
    f = file_ref.file()
    covers = []
    if hasattr(tag, "covers"):
        covers = tag.covers
    elif hasattr(tag, "pictureList"):
        covers = tag.pictureList()
    elif hasattr(f, "pictureList"):
        covers = f.pictureList()
    elif hasattr(f, "ID3v2Tag") and f.ID3v2Tag():
        covers = [
            a
            for a in f.ID3v2Tag().frameList()
            if isinstance(a, tagpy.id3v2.AttachedPictureFrame)
        ]
    if covers:
        cover = covers[0]
        fmt = tagpy.mp4.CoverArtFormats.Unknown
        if isinstance(cover, tagpy.mp4.CoverArt):
            return cover
        data = None
        if isinstance(cover, tagpy.ogg.flac.Picture):
            data = cover.data()
        else:
            data = cover.picture()
        mime = cover.mimeType().lower().strip()
        match mime:
            case "image/jpeg":
                fmt = tagpy.mp4.CoverArtFormats.JPEG
            case "image/png":
                fmt = tagpy.mp4.CoverArtFormats.PNG
            case "image/bmp":
                fmt = tagpy.mp4.CoverArtFormats.BMP
            case "image/gif":
                fmt = tagpy.mp4.CoverArtFormats.GIF
            case _:
                fmt = tagpy.mp4.CoverArtFormats.Unknown
        return tagpy.mp4.CoverArt(fmt, data)


class NIStemFile:
    STEM_DEFAULT_LABEL = [
        "drums",
        "bass",
        "other",
        "vocals",
    ]
    STEM_DEFAULT_COLOR = [
        "#009E73",
        "#D55E00",
        "#CC79A7",
        "#56B4E9",
    ]

    def __init__(
        self, path, codec: Codec, input_sample_rate: int, output_sample_rate: int
    ):
        self.__path = path
        self.__codec = codec
        self.__stream = StreamWriter(dst=path, format="mp4")

        self.__stream.add_audio_stream(
            sample_rate=input_sample_rate,
            num_channels=2,
            encoder=codec.encoder_name,
            encoder_sample_rate=output_sample_rate,
            encoder_num_channels=2,
            codec_config=CodecConfig(bit_rate=256000),
        )
        for i in range(4):
            self.__stream.add_audio_stream(
                sample_rate=input_sample_rate,
                num_channels=2,
                encoder=codec.encoder_name,
                encoder_sample_rate=output_sample_rate,
                encoder_num_channels=2,
                codec_config=CodecConfig(bit_rate=256000),
            )

    def __write_tensor_in_chunks(self, idx, tensor, progress):
        cursor = 0
        while cursor < tensor.shape[0]:
            chunk = torch.index_select(
                tensor,
                0,
                torch.arange(cursor, min(tensor.shape[0], cursor + CHUNK_SIZE)),
            )
            self.__stream.write_audio_chunk(idx, chunk)
            cursor += chunk.shape[0]
            progress.update(chunk.shape[0])

    def write(self, original, stems):
        sample_count = original.shape[1] + sum([t.shape[1] for t in stems.values()])

        match self.__codec:
            case Codec.FLAC:
                # Enable flac muxing in mp4
                muxer_options = {"strict": "-2"}
            case _:
                muxer_options = {}

        with self.__stream.open(option=muxer_options):
            with click.progressbar(
                length=sample_count, show_percent=True, label="Saving stems"
            ) as progress:
                self.__write_tensor_in_chunks(
                    0, torch.stack((original[0], original[1]), dim=1), progress
                )
                for key, tensor in stems.items():
                    self.__write_tensor_in_chunks(
                        self.STEM_DEFAULT_LABEL.index(key) + 1,
                        torch.stack((tensor[0], tensor[1]), dim=1),
                        progress,
                    )
                progress.finish()

    def update_metadata(self, src, **stem_metadata):
        # FIXME generating metadata atom after the file tags
        with stembox.Stem(self.__path) as f:
            f.stems = [
                dict(
                    color=stem_metadata.get(
                        f"stem_{i+1}_color",
                    )
                    or self.STEM_DEFAULT_COLOR[i],
                    name=stem_metadata.get(f"stem_{i+1}_label")
                    or self.STEM_DEFAULT_LABEL[i].title(),
                )
                for i in range(4)
            ]

        if not src:
            return

        try:
            src = tagpy.FileRef(src)
        except ValueError as e:
            logger.warn("Unable to read source tags for %s: %s", src, e)
        else:
            src = tagpy.FileRef(src)
            dst = tagpy.FileRef(self.__path)

            src_tag = src.tag()
            dst_tag = dst.tag()
            for tag in SUPPORTED_TAGS:
                setattr(dst_tag, tag, getattr(src_tag, tag))

            cover = _extract_cover(src)
            if cover:
                c = tagpy.mp4.CoverArtList()
                c.append(cover)
                dst_tag.covers = c
            dst.save()
