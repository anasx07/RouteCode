import os
import re
from typing import Optional, Dict, Tuple, Any

def parse_frontmatter(text: str) -> Tuple[Dict[str, str], str]:
    """
    Parses YAML-like frontmatter from a string.
    Expects format:
    ---
    key: value
    ---
    content
    
    Returns (metadata_dict, content_string).
    """
    pattern = r'^---\s*\n(.*?)\n---\s*\n?(.*)'
    match = re.match(pattern, text, re.DOTALL | re.MULTILINE)
    
    if not match:
        return {}, text
    
    frontmatter_raw = match.group(1)
    content = match.group(2)
    
    metadata = {}
    for line in frontmatter_raw.split('\n'):
        if ':' in line:
            key, value = line.split(':', 1)
            # Normalize: lowercase keys and strip quotes/whitespace from values
            metadata[key.strip().lower()] = value.strip().strip('"').strip("'")
            
    return metadata, content


def safe_resolve_path(file_path: str, workspace: Optional[str] = None) -> tuple:
    """Resolve a file path and validate it's within the workspace.
    
    Returns (resolved_absolute_path, error_message).
    On success, error_message is None.
    On failure, resolved_absolute_path is None.
    """
    ws = os.path.abspath(workspace or os.getcwd())
    try:
        joined = os.path.join(ws, file_path) if not os.path.isabs(file_path) else file_path
        resolved = os.path.abspath(os.path.normpath(joined))
        resolved = os.path.realpath(resolved)
    except (ValueError, OSError):
        return None, f"Invalid path: {file_path}"

    if not resolved.startswith(ws):
        return None, f"Path escapes workspace ({ws}): {file_path}"

    return resolved, None


TEXT_EXTENSIONS = {
    ".py", ".ts", ".tsx", ".js", ".jsx", ".rs", ".go", ".java",
    ".c", ".cpp", ".h", ".hpp", ".cs", ".rb", ".php", ".swift",
    ".kt", ".md", ".txt", ".json", ".yaml", ".yml",
    ".toml", ".cfg", ".ini", ".xml", ".html", ".css", ".scss",
    ".sql", ".sh", ".bat", ".ps1", ".env", ".gitignore", ".lock",
    ".pyw", ".r", ".m", ".mm", ".pl", ".pm", ".lua", ".scala",
    ".clj", ".ex", ".exs", ".hs", ".nim", ".zig", ".cjs", ".mjs",
    ".csv", ".log", ".conf", ".gradle", ".vue", ".svelte", ".astro",
    ".mts", ".cts"
}

IMAGE_EXTENSIONS = {".png", ".jpg", ".jpeg", ".gif", ".bmp", ".webp", ".svg"}

PDF_EXTENSIONS = {".pdf"}


def is_text_file(file_path: str) -> bool:
    _, ext = os.path.splitext(file_path)
    return ext.lower() in TEXT_EXTENSIONS or not ext


def parse_hex_color(hex_color: str) -> Tuple[int, int, int]:
    """
    Parses a hex color string (e.g. '#1a1a2e') into an (r, g, b) tuple.
    """
    try:
        h = hex_color.lstrip('#')
        if len(h) == 3:
            h = ''.join(c*2 for c in h)
        return tuple(int(h[i:i+2], 16) for i in (0, 2, 4))
    except (ValueError, IndexError):
        return (0, 0, 0)


def extract_tag(text: str, tag: str) -> Optional[str]:
    """Extracts content between <tag> and </tag>."""
    pattern = rf"<{tag}>(.*?)</{tag}>"
    match = re.search(pattern, text, re.DOTALL)
    return match.group(1).strip() if match else None


def strip_thought(text: str) -> Tuple[Optional[str], str]:
    """
    Separates the thought from the response.
    Returns (thought, response_without_thought).
    """
    thought = extract_tag(text, "thought")
    if thought is not None:
        pattern = r"<thought>.*?</thought>"
        clean_text = re.sub(pattern, "", text, flags=re.DOTALL).strip()
        return thought, clean_text
    
    if "<thought>" in text:
        parts = text.split("<thought>")
        if "</thought>" in parts[1]:
            t_parts = parts[1].split("</thought>")
            return t_parts[0].strip(), (parts[0] + t_parts[1]).strip()
        return parts[1].strip(), parts[0].strip()
        
    return None, text
