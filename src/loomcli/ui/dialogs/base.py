from ..console import _mirror_output

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
