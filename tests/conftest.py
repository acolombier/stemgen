import pytest
import stemgen.app
import stemgen.constant
import tempfile
import os


@pytest.fixture(scope="session")
def testdata_path():
    return f"{os.path.dirname(__file__)}/testdata"


@pytest.fixture(scope="session")
def generated_stem(testdata_path):
    with tempfile.TemporaryDirectory() as tmpdirname:
        stemgen.app.generate(
            files=[f"{testdata_path}/Oddchap - Sound 104.mp3"],
            output=tmpdirname,
            device="cpu",
            force=False,
            verbose=True,
            ext="stem.mp4",
            repo=None,
            model="htdemucs",
            shifts=1,
            overlap=0.25,
            jobs=1,
            codec=stemgen.constant.Codec.AAC,
            sample_rate=44100,
            drum_stem_label=None,
            drum_stem_color=None,
            bass_stem_label=None,
            bass_stem_color=None,
            other_stem_label=None,
            other_stem_color=None,
            vocal_stem_label=None,
            vocal_stem_color=None,
        )
        yield f"{tmpdirname}/Oddchap - Sound 104.stem.mp4"


@pytest.fixture(scope="session")
def created_stem(testdata_path):
    with tempfile.TemporaryDirectory() as tmpdirname:
        filename = f"{tmpdirname}/Oddchap - Sound 104.stem.mp4"
        stemgen.app.create(
            output=filename,
            mastered=f"{testdata_path}/Oddchap - Sound 104.mp3",
            drum=f"{testdata_path}/Oddchap - Sound 104.mp3",
            bass=f"{testdata_path}/Oddchap - Sound 104.mp3",
            other=f"{testdata_path}/Oddchap - Sound 104.mp3",
            vocal=f"{testdata_path}/Oddchap - Sound 104.mp3",
            force=False,
            verbose=True,
            codec=stemgen.constant.Codec.AAC,
            sample_rate=44100,
            drum_stem_label=None,
            drum_stem_color=None,
            bass_stem_label=None,
            bass_stem_color=None,
            other_stem_label=None,
            other_stem_color=None,
            vocal_stem_label=None,
            vocal_stem_color=None,
            copy_id3tags_from_mastered=True,
        )
        yield filename
