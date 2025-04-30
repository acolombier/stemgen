import pytest
import stemgen.app
import tempfile
import os
import ffmpeg
from stemgen.track import Track
from stemgen.constant import Codec
from stemgen.nistemfile import NIStemFile


@pytest.mark.parametrize("codec", Codec)
def test_codec(codec: Codec, testdata_path: str):
    src = f"{testdata_path}/Oddchap - Sound 104.mp3"
    with tempfile.TemporaryDirectory() as tmpdirname:
        dst = f"{tmpdirname}/Oddchap - Sound 104.stem.mp4"
        original = Track(src, 44100)
        stems = {
            "drums": Track(src, 44100),
            "bass": Track(src, 44100),
            "other": Track(src, 44100),
            "vocals": Track(src, 44100),
        }
        out = NIStemFile(dst, codec, 44100, 44100 if codec != Codec.OPUS else 48000)
        out.write(original.read(), {k: v.read() for k, v in stems.items()})

        streams = ffmpeg.probe(dst)["streams"]
        expected_codec = str(codec)
        assert [stream["codec_name"] for stream in streams] == [
            expected_codec for _ in range(5)
        ]
