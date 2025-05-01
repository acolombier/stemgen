from enum import IntEnum, StrEnum

MAX_STEM_LABEL_LENGTH = 32

# Used for file saving
CHUNK_SIZE = 1_024_000  # 1Mb


# See https://ffmpeg.org/doxygen/2.6/group__lavu__log__constants.html
class AvLog(IntEnum):
    """ffmpeg Logging Constants"""

    QUIET = -8
    PANIC = 0
    FATAL = 8
    ERROR = 16
    WARNING = 24
    INFO = 32
    VERBOSE = 40
    DEBUG = 48
    TRACE = 56


class Codec(StrEnum):
    AAC = "aac"
    ALAC = "alac"
    FLAC = "flac"
    OPUS = "opus"

    @property
    def encoder_name(self):
        # In case of opus, the ffmpeg encoder we want is called "libopus"
        # There is also one called only "opus", but it is experimental.
        match self:
            case Codec.OPUS:
                return "libopus"
            case _:
                return self.value


class SampleRate(IntEnum):
    Hz44100 = 44100
    Hz48000 = 48000
