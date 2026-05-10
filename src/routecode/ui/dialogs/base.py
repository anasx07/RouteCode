import asyncio
from typing import Any, Optional
from .manager import DialogManager


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
    Lifeycle management is delegated to DialogManager.
    Subclasses implement _build_container() and _get_focus_target().

    Key bindings resolve the dialog by calling self.future.set_result(value).
    """

    def __init__(self):
        self.future: Optional[asyncio.Future] = None

    def _build_container(self) -> Any:
        raise NotImplementedError

    def _get_focus_target(self) -> Any:
        raise NotImplementedError

    async def run_async(self) -> Any:
        self.future = asyncio.Future()
        return await DialogManager.run_dialog(
            container=self._build_container(),
            future=self.future,
            focus_target=self._get_focus_target(),
        )
