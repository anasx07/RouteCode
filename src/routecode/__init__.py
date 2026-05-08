try:
    try:
        from ._version import __version__
    except ImportError:
        from importlib.metadata import version

        __version__ = version("routecode")
except Exception:
    __version__ = "0.0.2"
