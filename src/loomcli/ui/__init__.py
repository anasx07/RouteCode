from .terminal import TerminalManager
from .theme import (
    THEMES, THEME_BACKGROUNDS, THEME_ACCENTS, 
    apply_theme, get_theme_bg, get_dialog_style
)
from .console import (
    console, mirror_console, _mirror_output,
    print_info, print_success, print_warning, print_error, print_step
)
from .dialogs import (
    LoomDialog, HoverRadioList, HoverCompletionsMenu, get_dialog_text, PaletteMenu, ModelPaletteMenu
)
from .renderables import (
    LoadingRenderable, LoomFace, LOOM_FACES, get_logo, 
    get_thinking_indicator, print_welcome_screen, 
    print_thought_elapsed, print_status_line, 
    get_tool_label, print_tool_call, print_tool_result, 
    print_session_stats, print_diff, refresh_screen
)

__all__ = [
    'TerminalManager', 'THEMES', 'THEME_BACKGROUNDS', 'THEME_ACCENTS', 
    'apply_theme', 'get_theme_bg', 'get_dialog_style', 'console', 
    'mirror_console', '_mirror_output', 'print_info', 'print_success', 
    'print_warning', 'print_error', 'print_step', 'LoomDialog', 
    'HoverRadioList', 'HoverCompletionsMenu', 'get_dialog_text', 
    'PaletteMenu', 'ModelPaletteMenu', 'LoadingRenderable', 'LoomFace', 
    'LOOM_FACES', 'get_logo', 'get_thinking_indicator', 'print_welcome_screen', 
    'print_thought_elapsed', 'print_status_line', 'get_tool_label', 
    'print_tool_call', 'print_tool_result', 'print_session_stats', 
    'print_diff', 'refresh_screen'
]

# Initialize the default theme
apply_theme("lava")
