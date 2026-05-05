import json
import asyncio
import httpx
from typing import AsyncGenerator, Dict, Any, Optional

class SSETransport:
    """
    Unified HTTP transport for AI providers with streaming support, 
    automatic retries, and unified error handling.
    """
    def __init__(self, timeout: float = 60.0, max_retries: int = 3):
        self.timeout = timeout
        self.max_retries = max_retries
        self.client = httpx.AsyncClient(timeout=timeout)

    def _get_delay(self, headers: httpx.Headers, attempt: int) -> float:
        """Calculates retry delay with exponential backoff and jitter."""
        import random
        retry_after = headers.get("retry-after")
        if retry_after:
            try:
                return float(retry_after)
            except ValueError:
                pass
        
        # 2^attempt (1, 2, 4...) with +/- 20% jitter
        base_delay = 2 ** attempt
        jitter = base_delay * 0.2
        return base_delay + random.uniform(-jitter, jitter)

    async def stream_post(self, url: str, headers: Dict[str, str], payload: Dict[str, Any]) -> AsyncGenerator[str, None]:
        """
        Executes a POST request and yields data from the SSE stream.
        Handles retries for rate limits (429), transient server errors (5xx),
        and connection/timeout issues.
        """
        for attempt in range(self.max_retries):
            try:
                async with self.client.stream("POST", url, headers=headers, json=payload) as response:
                    if response.status_code == 429 or response.status_code >= 500:
                        if attempt < self.max_retries - 1:
                            wait = self._get_delay(response.headers, attempt)
                            await asyncio.sleep(wait)
                            continue
                    
                    if response.status_code != 200:
                        await response.aread()
                        response.raise_for_status()

                    async for line in response.aiter_lines():
                        if not line:
                            continue
                        if line.startswith("data: "):
                            yield line[6:]
                    return  # Success, exit retry loop
                        
            except (httpx.TimeoutException, httpx.ConnectError):
                if attempt < self.max_retries - 1:
                    wait = self._get_delay(httpx.Headers(), attempt)
                    await asyncio.sleep(wait)
                    continue
                raise

    async def close(self):
        await self.client.aclose()
