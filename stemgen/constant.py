from enum import IntEnum, StrEnum

SAMPLE_RATE = 44100
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
