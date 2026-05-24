import os
import pathlib
import shutil

import sounddevice

pkg_dir = pathlib.Path(os.path.dirname(sounddevice.__file__))
bin_dir = pkg_dir / "_sounddevice_data" / "portaudio-binaries"
for f in list(bin_dir.glob("*arm64*")):
    f.unlink()
