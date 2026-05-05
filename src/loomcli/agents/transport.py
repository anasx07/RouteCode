import json
import time
import httpx
from typing import Generator, Dict, Any, Optional

class SSETransport:
    """
    Unified HTTP transport for AI providers with streaming support, 
    automatic retries, and unified error handling.
    """
    def __init__(self, timeout: float = 60.0, max_retries: int = 3):
        self.timeout = timeout
        self.max_retries = max_retries
        # Initialize client without HTTP2 by default for compatibility, 
        # but pooling is still beneficial.
        self.client = httpx.Client(timeout=timeout)

    def _parse_retry_after(self, headers: httpx.Headers, attempt: int) -> float:
        retry_after = headers.get("retry-after")
        if retry_after:
            try:
                return float(retry_after)
            except ValueError:
                pass
        return 2 ** attempt  # Exponential backoff

    def stream_post(self, url: str, headers: Dict[str, str], payload: Dict[str, Any]) -> Generator[str, None, None]:
        """
        Executes a POST request and yields data from the SSE stream.
        Handles retries for rate limits (429) and transient server errors (5xx).
        """
        for attempt in range(self.max_retries):
            try:
                with self.client.stream("POST", url, headers=headers, json=payload) as response:
                    if response.status_code == 429 or response.status_code >= 500:
                        if attempt < self.max_retries - 1:
                            wait = self._parse_retry_after(response.headers, attempt)
                            time.sleep(wait)
                            continue
                    
                    if response.status_code != 200:
                        # Non-retryable error or ran out of retries
                        error_body = response.read().decode()
                        yield json.dumps({
                            "type": "transport_error", 
                            "status_code": response.status_code, 
                            "body": error_body
                        })
                        return

                    for line in response.iter_lines():
                        if not line:
                            continue
                        # Standard SSE 'data: ' prefix handling
                        if line.startswith("data: "):
                            yield line[6:]
                        # Anthropic uses 'event: ' lines too, but providers usually 
                        # just care about 'data: '. We yield the raw line if it doesn't 
                        # start with data: to let providers handle specific SSE events?
                        # No, let's keep it simple: yield what follows 'data: '
                        # or yield the raw line if it's not a 'data: ' line but not empty?
                        # Actually, Anthropic needs 'data: ' lines.
                        
            except (httpx.TimeoutException, httpx.ConnectError) as e:
                if attempt < self.max_retries - 1:
                    time.sleep(2 ** attempt)
                    continue
                yield json.dumps({
                    "type": "transport_error", 
                    "error": str(e)
                })
                return

    def close(self):
        self.client.close()
