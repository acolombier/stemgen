import os
import sys
from pathlib import Path

from Cython.Build import cythonize
from setuptools import setup, Extension, find_packages

HERE = Path(__file__).resolve().parent

NAME = "stemgen"
DESCRIPTION = "STEM file generator library and utility."

URL = "https://github.com/acolombier/stemgen"
EMAIL = "stemgen@acolombier.dev"
AUTHOR = "Antoine Colombier"
REQUIRES_PYTHON = ">=3.11.0"

# Get version without explicitly loading the module.
for line in open("stemgen/__init__.py"):
    line = line.strip()
    if "__version__" in line:
        context = {}
        exec(line, context)
        VERSION = context["__version__"]


def load_requirements(name):
    required = [i.strip() for i in open(HERE / name)]
    required = [i for i in required if not i.startswith("#")]
    return required


REQUIRED = load_requirements("requirements.txt")


def extension_kwargs():
    default_taglib_path = HERE / "lib" / "taglib-cpp"
    taglib_install_dir = Path(os.environ.get("TAGLIB_HOME", str(default_taglib_path)))
    kwargs = dict()
    if sys.platform.startswith("win"):
        # on Windows, we compile static taglib build into the python module
        taglib_lib = taglib_install_dir / "lib" / "tag.lib"
        if not taglib_lib.exists():
            raise FileNotFoundError(f"{taglib_lib} not found")
        kwargs.update(
            define_macros=[("TAGLIB_STATIC", None)],
            extra_objects=[str(taglib_lib)],
            include_dirs=[str(taglib_install_dir / "include")],
        )
    else:
        # On unix systems, use the dynamic library. Still, add the (default) TAGLIB_HOME
        # to allow overriding system taglib with custom build.
        kwargs.update(
            libraries=["tag"],
            extra_compile_args=["-std=c++20"],
            extra_link_args=["-std=c++20"],
            language="c++",
        )
    if os.getenv("PREFIX"):
        prefix = os.environ["PREFIX"]
        kwargs.update(
            include_dirs=[f"{prefix}/include/"],
            library_dirs=[
                f"{prefix}/lib/",
            ],
        )

    return kwargs


try:
    with open(HERE / "README.md", encoding="utf-8") as f:
        long_description = "\n" + f.read()
except FileNotFoundError:
    long_description = DESCRIPTION

setup(
    name=NAME,
    version=VERSION,
    description=DESCRIPTION,
    long_description=long_description,
    long_description_content_type="text/markdown",
    author=AUTHOR,
    author_email=EMAIL,
    python_requires=REQUIRES_PYTHON,
    url=URL,
    install_requires=REQUIRED,
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
    ],
    packages=find_packages(),
    entry_points={
        "console_scripts": [
            "stemgen = stemgen.__main__:main",
        ]
    },
    ext_modules=cythonize(
        [
            Extension(
                "stembox",
                [
                    str(HERE / "stembox/stembox.pyx"),
                    str(HERE / "stembox/stembox_api.cpp"),
                ],
                **extension_kwargs(),
            )
        ],
        language_level="3",
        force=True,
    ),
)
