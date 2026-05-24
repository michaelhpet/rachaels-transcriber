from PyInstaller.utils.hooks import collect_data_files, collect_dynamic_libs

datas = collect_data_files("sounddevice")
datas = [(s, d) for s, d in datas if "arm64" not in s.lower()]

binaries = collect_dynamic_libs("sounddevice")
binaries = [(s, d) for s, d in binaries if "arm64" not in s.lower()]
