import sys
from pathlib import Path


def get_resource_path(relative_path: str) -> Path:
    """Get absolute path to resource, works for dev and for PyInstaller."""
    try:
        # PyInstaller creates a temp folder and stores path in _MEIPASS
        base_path = Path(sys._MEIPASS)
    except Exception:
        # In development, look relative to the routecode package root
        # src/routecode/utils/paths.py -> .parent.parent is src/routecode/
        base_path = Path(__file__).parent.parent

    # If the relative path doesn't start with routecode, but we are in MEIPASS,
    # we might need to prepend it depending on how datas was structured.
    # With collect_data_files('routecode'), it's usually at sys._MEIPASS / 'routecode'

    full_path = base_path / relative_path

    # Fallback check for PyInstaller internal structure
    if not full_path.exists() and hasattr(sys, "_MEIPASS"):
        alt_path = base_path / "routecode" / relative_path
        if alt_path.exists():
            return alt_path

    return full_path
