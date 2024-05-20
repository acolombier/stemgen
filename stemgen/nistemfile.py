import click

import taglib
from torchaudio.io import StreamWriter, CodecConfig
import stembox
import torch

from .constant import SAMPLE_RATE, CHUNK_SIZE


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

    def __init__(self, path, use_alac=False):
        self.__path = path
        self.__stream = StreamWriter(dst=path, format="mp4")

        self.__stream.add_audio_stream(
            sample_rate=SAMPLE_RATE,
            num_channels=2,
            encoder="alac" if use_alac else "aac",
            encoder_sample_rate=SAMPLE_RATE,
            encoder_num_channels=2,
            codec_config=CodecConfig(bit_rate=256000),
        )
        for i in range(4):
            self.__stream.add_audio_stream(
                sample_rate=SAMPLE_RATE,
                num_channels=2,
                encoder="alac" if use_alac else "aac",
                encoder_sample_rate=SAMPLE_RATE,
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
        with self.__stream.open():
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
        with taglib.File(src) as src, taglib.File(
            self.__path, save_on_exit=True
        ) as dst:
            dst.tags = src.tags

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
