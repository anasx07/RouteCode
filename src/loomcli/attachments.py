import os
import base64
from pathlib import Path
from typing import Dict, Optional


TEXT_EXTENSIONS = {".py", ".ts", ".js", ".jsx", ".tsx", ".rs", ".go", ".java", ".c", ".cpp",
                   ".h", ".hpp", ".cs", ".rb", ".php", ".swift", ".kt", ".md", ".txt",
                   ".json", ".yaml", ".yml", ".toml", ".cfg", ".ini", ".xml", ".html",
                   ".css", ".scss", ".sql", ".sh", ".bat", ".ps1", ".env",
                   ".csv", ".log", ".cfg", ".conf", ".gradle", ".lock", ".vue", ".svelte",
                   ".astro", ".mjs", ".cjs", ".mts", ".cts"}

IMAGE_EXTENSIONS = {".png", ".jpg", ".jpeg", ".gif", ".bmp", ".webp", ".svg"}

PDF_EXTENSION = {".pdf"}


def get_attachment_type(path: str) -> str:
    ext = os.path.splitext(path)[1].lower()
    if ext in IMAGE_EXTENSIONS:
        return "image"
    if ext in PDF_EXTENSION:
        return "pdf"
    if ext in TEXT_EXTENSIONS or not ext:
        return "text"
    return "binary"


def load_attachment(path: str) -> Optional[Dict]:
    """Load a file as an attachment. Returns dict with type, name, content."""
    if not os.path.exists(path):
        return None

    resolved = os.path.abspath(path)
    name = os.path.basename(resolved)
    att_type = get_attachment_type(resolved)

    try:
        if att_type == "image":
            with open(resolved, "rb") as f:
                data = base64.b64encode(f.read()).decode("utf-8")
            ext = os.path.splitext(resolved)[1].lstrip(".")
            mime = f"image/{ext}" if ext != "jpg" else "image/jpeg"
            return {
                "type": "image",
                "name": name,
                "mime_type": mime,
                "data": data,
                "path": resolved,
                "size": len(data)
            }

        elif att_type == "pdf":
            content = f"[PDF file: {name}. Text extraction requires PyMuPDF. File at: {resolved}]"
            return {
                "type": "text",
                "name": name,
                "content": content,
                "path": resolved
            }

        elif att_type == "text":
            with open(resolved, "r", encoding="utf-8", errors="replace") as f:
                content = f.read()
            MAX_TEXT = 50000
            if len(content) > MAX_TEXT:
                content = content[:MAX_TEXT] + f"\n... [truncated at {MAX_TEXT} chars]"
            return {
                "type": "text",
                "name": name,
                "content": content,
                "path": resolved
            }

        else:
            return {
                "type": "binary",
                "name": name,
                "content": f"[Binary file: {name} at {resolved}]",
                "path": resolved
            }

    except Exception as e:
        return {
            "type": "error",
            "name": name,
            "content": f"Error loading {name}: {e}",
            "path": resolved
        }
