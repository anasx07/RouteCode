import io
from typing import Optional
from rich.console import Console


class ConsoleProxy:
    """
    A proxy for the Rich Console that allows the underlying instance to be
    swapped (e.g., during theme changes) without breaking existing references.
    """

    def __init__(self):
        self._instance: Optional[Console] = None

    def set_instance(self, instance: Console):
        self._instance = instance

    def __getattr__(self, name):
        if self._instance is None:
            # Fallback for early access during initialization
            self._instance = Console()
        return getattr(self._instance, name)

    def __enter__(self):
        return self._instance.__enter__()

    def __exit__(self, *args):
        return self._instance.__exit__(*args)

    def get_print_method(self):
        """Returns the underlying console's print method.
        Use this instead of accessing _instance directly when you need
        the actual console's print (e.g., after a theme change)."""
        inst = self._instance or Console()
        return inst.print


# Global console proxy instances
console = ConsoleProxy()
mirror_console = ConsoleProxy()

# Internal state for terminal mirroring
_mirror_output = io.StringIO()


def print_info(message: str):
    console.print(f"[info]ℹ[/info] {message}")


def print_success(message: str):
    console.print(f"[success]✔[/success] {message}")


def print_warning(message: str):
    console.print(f"[warning]⚠[/warning] {message}")


def print_error(message: str):
    console.print(f"[error]✘[/error] {message}")


def print_step(message: str):
    console.print(f"[accent]➤[/accent] {message}")
