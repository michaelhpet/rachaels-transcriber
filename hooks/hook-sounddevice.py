from PyInstaller.utils.hooks import collect_data_files

datas = collect_data_files("sounddevice")
datas = [(s, d) for s, d in datas if "arm64" not in s.lower()]
