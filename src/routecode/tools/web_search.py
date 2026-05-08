import httpx
from typing import Any, Dict, Optional, TYPE_CHECKING
from pydantic import BaseModel, Field
from .base import BaseTool
import urllib.parse

if TYPE_CHECKING:
    from ..core import RouteCodeContext


class WebSearchInput(BaseModel):
    query: str = Field(..., description="The search query")
    num_results: int = Field(5, description="Number of results to return (default 5)")


class WebSearchTool(BaseTool):
    name = "google_web_search"
    description = "Search the web for a query and return relevant titles and URLs. Use for research, finding documentation, or latest news."
    input_schema = WebSearchInput
    isConcurrencySafe = True
    isReadOnly = True

    def prompt(self) -> str:
        return "- google_web_search: Search the web and return relevant results. Read-only, concurrency-safe."

    def get_activity_description(self, query: str = "", **kwargs) -> str:
        return f"Searching: {query[:40]}"

    def _run(
        self,
        query: str,
        num_results: int = 5,
        ctx: Optional["RouteCodeContext"] = None,
        provider: Optional[Any] = None,
        **kwargs,
    ) -> Dict[str, Any]:
        try:
            # We use DuckDuckGo as a reliable free fallback for "google_web_search"
            # It doesn't require an API key and is easy to parse.
            search_url = (
                f"https://html.duckduckgo.com/html/?q={urllib.parse.quote(query)}"
            )
            headers = {
                "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36"
            }

            response = httpx.get(search_url, headers=headers, timeout=15.0)
            if response.status_code != 200:
                return {
                    "success": False,
                    "error": f"Search failed with status {response.status_code}",
                }

            # Simple HTML parsing using basic string manipulation (to avoid heavy bs4 dependency if not present)
            text = response.text
            results = []

            # Split by result containers
            blocks = text.split('class="result__body"')
            for block in blocks[1 : num_results + 1]:
                try:
                    # Extract title
                    title_start = block.find('class="result__a"')
                    if title_start == -1:
                        continue
                    title_start = block.find(">", title_start) + 1
                    title_end = block.find("</a>", title_start)
                    title = block[title_start:title_end].strip()
                    # Remove HTML tags from title
                    import re

                    title = re.sub("<[^<]+?>", "", title)

                    # Extract URL
                    url_start = block.find('href="')
                    if url_start == -1:
                        continue
                    url_start += 6
                    url_end = block.find('"', url_start)
                    url = block[url_start:url_end]
                    # DDG often uses redirects, let's clean them if needed
                    if url.startswith("//"):
                        url = "https:" + url
                    if "uddg=" in url:
                        url = urllib.parse.unquote(url.split("uddg=")[1].split("&")[0])

                    # Extract Snippet
                    snippet_start = block.find('class="result__snippet"')
                    if snippet_start == -1:
                        continue
                    snippet_start = block.find(">", snippet_start) + 1
                    snippet_end = block.find("</a>", snippet_start)
                    snippet = block[snippet_start:snippet_end].strip()
                    snippet = re.sub("<[^<]+?>", "", snippet)

                    results.append({"title": title, "url": url, "snippet": snippet})
                except Exception:
                    continue

            if not results:
                return {"success": True, "results": [], "message": "No results found."}

            return {"success": True, "results": results}

        except Exception as e:
            return {"success": False, "error": f"Search tool error: {str(e)}"}
