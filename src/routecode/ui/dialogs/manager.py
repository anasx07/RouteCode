"""
Centralized dialog lifecycle management.

DialogManager handles Float injection, focus save/restore, modal state,
mouse tracking, and cleanup — so individual dialog classes only need to
implement their layout.
"""

import asyncio
from typing import Any, Optional
from prompt_toolkit.layout.containers import Float, Container
from prompt_toolkit.application.current import get_app
from ..terminal import TerminalManager


class DialogManager:
    """
    Manages the lifecycle of a modal dialog Float.

    Usage:
        future = asyncio.Future()
        result = await DialogManager.run_dialog(
            container=dialog_container,
            future=future,
            focus_target=search_field,
        )
    """

    @staticmethod
    async def run_dialog(
        container: Container,
        future: Optional["asyncio.Future[Any]"] = None,
        focus_target: Any = None,
    ) -> Any:
        """
        Injects a dialog Float, awaits a future, cleans up.

        The dialog signals completion by calling:
            future.set_result(value)

        If no future is provided, one is created internally. In that case,
        the dialog must have access to resolve it another way (e.g. via
        an on_open callback).
        """
        future = future or asyncio.Future()
        app = get_app()

        if not app or not app.is_running or not hasattr(app.layout.container, "floats"):
            raise RuntimeError("Dialog must be run within an active Application with floats")

        dialog_float = Float(content=container, transparent=False)
        app.layout.container.floats.append(dialog_float)
        previous_focus = app.layout.current_window

        if focus_target:
            app.layout.focus(focus_target)

        if hasattr(app, "routecode_repl"):
            app.routecode_repl.is_modal_open = True
            app.routecode_repl.update_style()

        app.invalidate()
        TerminalManager.enable_mouse_tracking()

        try:
            return await future
        finally:
            TerminalManager.disable_mouse_tracking()
            if dialog_float in app.layout.container.floats:
                app.layout.container.floats.remove(dialog_float)
            if previous_focus:
                try:
                    app.layout.focus(previous_focus)
                except Exception:
                    pass
            if hasattr(app, "routecode_repl"):
                app.routecode_repl.is_modal_open = False
                app.routecode_repl.update_style()
            app.invalidate()
