from demucs.audio import convert_audio_channels
import ffmpeg
import logging
from torch import tensor
import numpy as np

from .constant import SAMPLE_RATE

logger = logging.getLogger(__file__)


class Track:
    def __init__(self, path):
        probe = ffmpeg.probe(path)
        self.__stream = [s for s in probe["streams"] if s["codec_type"] == "audio"]

        if not self.__stream:
            raise ValueError("No audio stream available in the file")
        elif len(self.__stream) > 1:
            logger.warn("Found more than one audio stream in file. Using the first one")

        self.__stream = self.__stream[0]
        self.__path = path

    @property
    def audio_channels(self):
        return int(self.__stream["channels"])

    def read(self):
        out, _ = (
            ffmpeg.input(self.__path)
            .output("-", format="f32le", ar=SAMPLE_RATE, ac=2)
            .run(capture_stdout=True, capture_stderr=True)
        )
        wav = tensor(np.frombuffer(out, dtype=np.float32))
        return convert_audio_channels(
            wav.view(-1, self.audio_channels).t(), self.audio_channels
        )
