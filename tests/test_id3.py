import pytest
import ffmpeg
import cv2
import numpy as np


@pytest.mark.parametrize(
    "filename",
    [
        ("generated_stem"),
        ("created_stem"),
    ],
)
def test_ensure_id3(filename: str, request):
    filename = request.getfixturevalue(filename)
    metadata = ffmpeg.probe(filename)["format"]["tags"]

    assert metadata["title"] == "Sound 104"
    assert metadata["artist"] == "Odd Chap"
    assert metadata["comment"] == (
        "Spotify - https://open.spotify.com/track/2481amjn69XrXmgFIvBmbD?si=c5a62dce53744656\n"
        "Bandcamp - https://oddchap.bandcamp.com/album/sound-104\n"
        "Apple - https://music.apple.com/nz/album/sound-104/1691470758?i=1691470760\n"
        "Amazon - https://music.amazon.com.au/albums/B0C7BW18ZG\n"
        '\nOH YEAHHHH! It\'s finally here, I was originally going to keep the "sound 10_" '
        "series to 3 parts with the exception of doing number 4 only if I found the perfect "
        "audio dialogue sample to use. And well I found it and here we are! Sound 104, little "
        "less heavy than the other in the series but ive upped the funkiness!"
    )
    assert metadata["genre"] == "Electro Swing"


@pytest.mark.parametrize(
    "filename",
    [
        ("generated_stem"),
        ("created_stem"),
    ],
)
def test_ensure_coverart(filename: str, testdata_path: str, request):
    filename = request.getfixturevalue(filename)
    out, _ = ffmpeg.input(filename).output("-", format="apng").run(capture_stdout=True)
    actual = cv2.imdecode(np.frombuffer(out, np.uint8), cv2.IMREAD_COLOR)

    with open(f"{testdata_path}/rocket.png", "rb") as f:
        expected = cv2.imdecode(np.frombuffer(f.read(), np.uint8), cv2.IMREAD_COLOR)

        assert actual.shape == expected.shape, f"{len(data)} != {len(out)}"
        for x in range(512):
            for y in range(512):
                assert tuple(expected[y, x]) == tuple(
                    actual[y, x]
                ), f"Mismatch at pixel x={x}, y={y}"
