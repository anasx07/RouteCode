try:
    try:
        from ._version import __version__
    except ImportError:
        from importlib.metadata import version

        __version__ = version("loomcli")
except Exception:
    __version__ = "1.1.0"
