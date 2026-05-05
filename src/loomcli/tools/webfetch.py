from typing import Any, Dict
from pydantic import BaseModel, Field
from .base import BaseTool


class WebFetchInput(BaseModel):
    url: str = Field(..., description="The URL to fetch content from")


class WebFetchTool(BaseTool):
    name = "webfetch"
    description = "Fetch content from a URL and return it as markdown. Use for reading documentation, APIs, or web pages."
    input_schema = WebFetchInput
    isConcurrencySafe = True
    isReadOnly = True

    def prompt(self) -> str:
        return "- webfetch: Fetch a URL and return its content as markdown. Read-only, concurrency-safe."

    def get_activity_description(self, url: str = "", **kwargs) -> str:
        return f"WebFetch({url[:40]})"

    def execute(self, url: str) -> Dict[str, Any]:
        try:
            import httpx
            response = httpx.get(url, follow_redirects=True, timeout=30.0)
            if response.status_code != 200:
                return {"success": False, "error": f"HTTP {response.status_code}: {response.text[:200]}"}

            content_type = response.headers.get("content-type", "")
            text = response.text

            if "text/html" in content_type or "text/plain" in content_type:
                # Try to extract readable content from HTML
                if "text/html" in content_type:
                    try:
                        from html.parser import HTMLParser
                        class TextExtractor(HTMLParser):
                            def __init__(self):
                                super().__init__()
                                self._text = []
                                self._skip = False
                            def handle_starttag(self, tag, attrs):
                                if tag in ("script", "style"):
                                    self._skip = True
                            def handle_endtag(self, tag):
                                if tag in ("script", "style"):
                                    self._skip = False
                            def handle_data(self, data):
                                if not self._skip:
                                    self._text.append(data.strip())
                            def get_text(self):
                                return "\n".join(t for t in self._text if t)

                        extractor = TextExtractor()
                        extractor.feed(text)
                        text = extractor.get_text()[:50000]
                    except Exception:
                        text = text[:50000]
                else:
                    text = text[:50000]

                return {"success": True, "content": text, "url": url}
            elif "application/json" in content_type:
                return {"success": True, "content": text[:50000], "url": url}
            else:
                return {"success": True, "content": text[:50000], "url": url}

        except Exception as e:
            return {"success": False, "error": f"Failed to fetch {url}: {str(e)}"}
