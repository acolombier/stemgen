from demucs.separate import Separator
import click
import warnings


from .constant import SAMPLE_RATE


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

    def callback(self, data):
        if (
            data.get("state") == "start"
            or data.get("segment_offset") is None
            or not data.get("progress")
        ):
            return
        global_offset = data["segment_offset"]
        progress = data["progress"]
        if progress:
            if not global_offset and self.__last_offset:
                offset = (
                    progress.length / len(self.__separator.model.weights)
                    - self.__last_offset
                )
                self.__last_offset = 0
            else:
                offset = data["segment_offset"] - self.__last_offset
                self.__last_offset = data["segment_offset"]
            progress.update(offset)

    def run(self, samples, verbose=False):
        with click.progressbar(
            length=samples.shape[1] * len(self.__separator.model.weights),
            show_eta=True,
            show_percent=True,
            label="Demucsing",
        ) as progress, warnings.catch_warnings(record=True) as warn:
            self.__separator.update_parameter(callback_arg=dict(progress=progress))
            try:
                return self.__separator.separate_tensor(samples, SAMPLE_RATE)
            finally:
                progress.update(progress.length - progress.pos)
                progress.finish()
                if verbose:
                    if len(warn) > 0:
                        click.secho(
                            f"\nThe following warnings were captured while demucs was processing:",
                            fg="yellow",
                        )

                    for message in warn:
                        click.secho(
                            f"\t{warnings._formatwarnmsg_impl(message)}", fg="yellow"
                        )
