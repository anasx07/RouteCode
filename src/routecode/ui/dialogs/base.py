from ..console import _mirror_output
import asyncio
from typing import Any, Optional
from prompt_toolkit.application.current import get_app
from prompt_toolkit.layout.containers import Float
from ..terminal import TerminalManager


def _get_backdrop_ansi() -> str:
    """Generates a dimmed ANSI screenshot of the current terminal state."""
    full_ansi = _mirror_output.getvalue()
    from prompt_toolkit.output.defaults import create_output

    try:
        h = create_output().get_size().rows
    except Exception:
        import shutil

        h = shutil.get_terminal_size().lines
    lines = full_ansi.splitlines()
    recent_lines = lines[-h:]
    ansi_content = "\n".join(recent_lines)
    # Strong dimming for the backdrop
    return f"\033[2m\033[38;5;238m{ansi_content}\033[0m"


def get_dialog_text(main_text: str, dialog_type: str = "radio") -> str:
    """Returns formatted text with keyboard guides for standard dialogs."""
    guides = {
        "radio": "\n\n[ Tab ] Focus Buttons  [ ↑↓ ] Select  [ Enter ] Confirm",
        "button": "\n\n[ ←/→ ] Switch Buttons  [ Enter ] Select",
        "input": "\n\n[ ↑/↓ ] Focus Buttons  [ Enter ] Submit",
        "message": "\n\n[ Enter ] OK",
    }
    guide = guides.get(dialog_type, guides["radio"])
    return f"{main_text}{guide}"


class BaseModalLayer:
    """
    Abstract base class for modal dialogs and overlays.
    Provides standard lifecycle management for Float injection,
    mouse tracking, focus trapping, global dimming state, and cleanup.
    """

    def __init__(self):
        self.future: Optional[asyncio.Future] = None

    def _build_container(self) -> Any:
        """Override to return the prompt_toolkit container (e.g., Shadow) to inject."""
        raise NotImplementedError

    def _get_focus_target(self) -> Any:
        """Override to return the widget that should receive focus initially."""
        raise NotImplementedError

    async def run_async(self) -> Any:
        import asyncio

        current_app = get_app()
        is_injected = (
            current_app
            and current_app.is_running
            and hasattr(current_app.layout.container, "floats")
        )

        if not is_injected:
            # Fallback for headless testing or non-injected states
            raise RuntimeError(
                "BaseModalLayer must be run within an active Application with floats."
            )

        self.future = asyncio.Future()
        menu_container = self._build_container()
        focus_target = self._get_focus_target()

        menu_float = Float(content=menu_container, transparent=False)
        current_app.layout.container.floats.append(menu_float)
        previous_focus = current_app.layout.current_window

        if focus_target:
            current_app.layout.focus(focus_target)

        if hasattr(current_app, "routecode_repl"):
            current_app.routecode_repl.is_modal_open = True
            current_app.routecode_repl.update_style()

        current_app.invalidate()
        TerminalManager.enable_mouse_tracking()

        try:
            return await self.future
        finally:
            TerminalManager.disable_mouse_tracking()
            if menu_float in current_app.layout.container.floats:
                current_app.layout.container.floats.remove(menu_float)
            if previous_focus:
                try:
                    current_app.layout.focus(previous_focus)
                except Exception:
                    pass
            if hasattr(current_app, "routecode_repl"):
                current_app.routecode_repl.is_modal_open = False
                current_app.routecode_repl.update_style()
            current_app.invalidate()
