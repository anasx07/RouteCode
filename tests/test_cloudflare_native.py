import asyncio
import os
import json
import sys
import logging
from pathlib import Path
from typing import List, Dict, Any, Optional

# Add src to path
sys.path.append(str(Path(__file__).parent.parent / "src"))

from routecode.agents.cloudflare_provider import CloudflareProvider
from routecode.utils.logger import setup_logging

async def test_cloudflare(model_id: Optional[str] = None):
    setup_logging(logging.DEBUG)
    api_key = os.environ.get("CLOUDFLARE_API_KEY")
    account_id = os.environ.get("CLOUDFLARE_ACCOUNT_ID")
    
    if not api_key or not account_id:
        print("Error: CLOUDFLARE_API_KEY and CLOUDFLARE_ACCOUNT_ID environment variables must be set.")
        return

    print(f"Testing Cloudflare Native Provider...")
    print(f"Account ID: {account_id}")
    
    provider = CloudflareProvider(api_key, account_id=account_id)
    
    if not model_id:
        # Test model list
        print("\nFetching models...")
        models = await provider.get_models()
        print(f"Found {len(models)} models.")
        
        # Default model selection
        model_id = "@cf/meta/llama-3-8b-instruct"
        if any("moonshot" in m["id"] for m in models):
            # Sort to get highest version first or 2.6 specifically
            moonshots = [m["id"] for m in models if "moonshot" in m["id"]]
            moonshots.sort(reverse=True)
            model_id = moonshots[0]
            if any("2.6" in m for m in moonshots):
                model_id = [m for m in moonshots if "2.6" in m][0]
        
    print(f"\nTesting streaming ask with model: {model_id}")
    messages = [
        {"role": "system", "content": "You are a helpful assistant."},
        {"role": "user", "content": "Hello! Reply with a short joke."}
    ]
    
    full_response = ""
    print("Response: ", end="", flush=True)
    async for chunk in provider.ask(messages, model_id, stream=True):
        if chunk["type"] == "text":
            print(chunk["content"], end="", flush=True)
            full_response += chunk["content"]
        elif chunk["type"] == "error":
            print(f"\nError: {chunk['content']}")
            break
            
    print("\n\nTest Complete.")
    if full_response:
        print("Status: SUCCESS")
    else:
        print("Status: FAILED (Empty response)")

if __name__ == "__main__":
    mid = sys.argv[1] if len(sys.argv) > 1 else None
    asyncio.run(test_cloudflare(mid))
