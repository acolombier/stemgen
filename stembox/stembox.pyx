# distutils: language = c++

import json
import re
import os

from stembox cimport File as CFile

cdef class Stem:
    cdef CFile c_file
    cdef object __data

    def __init__(self, path):
        if not os.path.exists(path):
            raise FileNotFoundError(f"No such file: {path}")
        self.c_file = CFile(path.encode('utf-8'))
        self.__data = self.c_file.data()
        if self.__data:
            self.__data = json.loads(self.__data.decode('utf-8'))
        else:
            self.__data = {}

        if not self.__data.get("stems"):
            self.__data["stems"] = []
        if not self.__data.get("mastering_dsp"):
            self.__data["mastering_dsp"] = {
                "compressor": {
                    "enabled": False,
                    "ratio": 10,
                    "output_gain": 0,
                    "release": 1.0,
                    "attack": 0.0001,
                    "input_gain": 0,
                    "threshold": 0,
                    "hp_cutoff": 20,
                    "dry_wet": 100
                },
                "limiter": {
                    "enabled": False,
                    "release": 1.0,
                    "threshold": 0,
                    "ceiling": 0
                }
            }
        if not self.__data.get("version"):
            self.__data["version"] = 1

    def save(self):
        if not self.__data["stems"]:
            self.c_file.setData("".encode('UTF-8'))
        else:
            self._validate_stems(self.stems)
            self.c_file.setData(json.dumps(self.__data).encode('utf-8'))
        return self.c_file.save()

    def _validate_stems(self, value):
        if not isinstance(value, list):
            raise ValueError("expected a list of stem")
        for idx, stem in enumerate(value):
            if "color" not in stem.keys():
                raise ValueError(f"Missing 'color' for stem #{idx}")
            elif not isinstance(stem["color"], str) or not re.match("^#[0-9a-f]{6}$", stem["color"], flags=re.IGNORECASE):
                raise ValueError(f"Invalid 'color' for stem #{idx}")
            if "name" not in stem.keys():
                raise ValueError(f"Missing 'name' for stem #{idx}")
            elif not isinstance(stem["name"], str):
                raise ValueError(f"Invalid 'name' for stem #{idx}")
            if len(stem.keys()) != 2:
                raise ValueError(f"Superfluous property found for stem #{idx}")

    @property
    def stems(self):
        return self.__data["stems"]

    @stems.setter
    def stems(self, value):
        self._validate_stems(value)
        self.__data["stems"] = value

    @property
    def version(self):
        return self.__data["version"]

    @version.setter
    def version(self, value):
        if not isinstance(value, int) or value < 0:
            raise ValueError("expected a positive integer")
        self.__data["version"] = value

    @property
    def mastering_dsp(self):
        return self.__data["mastering_dsp"]

    @mastering_dsp.setter
    def mastering_dsp(self, value):
        # TODO validation
        self.__data = value

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_value, exc_traceback):
        if not self.save():
            raise IOError("Unable to save the file")
