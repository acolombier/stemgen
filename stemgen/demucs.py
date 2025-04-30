from demucs.separate import Separator
import warnings


class Demucs:
    def __init__(self, repo, model, device, shifts=1, overlap=0.25, jobs=1):
        self.__separator = Separator(
            repo=repo,
            model=model,
            device=device,
            shifts=shifts,
            split=True,
            overlap=overlap,
            progress=False,
            jobs=jobs,
            segment=None,
            callback=self.callback,
        )
        self.__last_offset = 0

    @property
    def weights(self):
        return self.__separator.model.weights

    @property
    def sample_rate(self):
        return self.__separator.samplerate

    def callback(self, data):
        if (
            data.get("state") == "start"
            or data.get("segment_offset") is None
            or not data.get("progress")
        ):
            return
        global_offset = data["segment_offset"]
        length = data["length"]
        progress = data["progress"]
        if progress:
            if not global_offset and self.__last_offset:
                offset = (
                    length / len(self.__separator.model.weights) - self.__last_offset
                )
                self.__last_offset = 0
            else:
                offset = data["segment_offset"] - self.__last_offset
                self.__last_offset = data["segment_offset"]
            progress(offset)

    def length(self, samples):
        return samples.shape[1] * len(self.__separator.model.weights)

    def run(self, samples, update_cb=lambda x: None, finish_cb=lambda: None):
        self.__separator.update_parameter(
            callback_arg=dict(length=self.length(samples), progress=update_cb)
        )
        with warnings.catch_warnings(record=True) as warn:
            try:
                return (
                    *self.__separator.separate_tensor(
                        samples, self.__separator.samplerate
                    ),
                    warn,
                )
            finally:
                update_cb(self.length(samples))
                finish_cb()
