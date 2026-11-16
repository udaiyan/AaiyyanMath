# Matplotlib export helper for reports (used from trusted Python blocks).
import aaiyyan_runtime as aaiyyan


def save_series(series, path, title=""):
    return aaiyyan.export_figure(series, path, title=title)
