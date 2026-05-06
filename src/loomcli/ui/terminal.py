import sys
import os
import shutil
import atexit
import signal
from ..utils import parse_hex_color

def _cleanup_terminal():
    """Ensure terminal is left in a clean state (e.g., mouse tracking disabled)."""
    TerminalManager.disable_mouse_tracking()

atexit.register(_cleanup_terminal)

# Handle termination signals to ensure cleanup
def _signal_handler(signum, frame):
    _cleanup_terminal()
    sys.exit(signum)

try:
    signal.signal(signal.SIGTERM, _signal_handler)
except (AttributeError, ValueError):
    # Some platforms or environments don't support SIGTERM
    pass

class TerminalManager:
    """Unified manager for low-level terminal manipulation and state."""
    
    @staticmethod
    def clear():
        """Clears the screen and resets cursor position using ANSI escapes."""
        sys.stdout.write("\033[2J\033[H")
        sys.stdout.flush()

    @staticmethod
    def set_background(bg_color: str):
        """Sets terminal background color via OSC 11 and palette 0."""
        try:
            r, g, b = parse_hex_color(bg_color)
            # OSC 11: Set background color
            sys.stdout.write(f"\033]11;rgb:{r:02x}/{g:02x}/{b:02x}\033\\")
            # OSC 4;0: Set palette color 0 (often used for margins/padding)
            sys.stdout.write(f"\033]4;0;rgb:{r:02x}/{g:02x}/{b:02x}\033\\")
            sys.stdout.flush()
        except Exception:
            pass

    @staticmethod
    def get_size():
        """Returns terminal dimensions (rows, cols)."""
        return shutil.get_terminal_size()

    @staticmethod
    def enable_mouse_tracking():
        """Enables Any Event mouse tracking."""
        sys.stdout.write("\033[?1003h")
        sys.stdout.flush()

    @staticmethod
    def disable_mouse_tracking():
        """Disables Any Event mouse tracking."""
        sys.stdout.write("\033[?1003l")
        sys.stdout.flush()
