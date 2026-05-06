import json
import time
import asyncio
from typing import Any, Dict, List, Optional, Callable
from .context import LoomContext
from .state import count_tokens
from .tools import registry
from .agents.registry import PROVIDER_MAP

class OrchestratorHooks:
    """Hooks for the AgentOrchestrator to signal progress and updates."""
    async def on_chunk(self, chunk: Dict[str, Any]):
        """Called for every chunk received from the provider."""
        pass

    async def on_tool_call(self, name: str, args: Dict[str, Any]):
        """Called before a tool is executed."""
        pass

    async def on_tool_result(self, name: str, result: Dict[str, Any], elapsed: float):
        """Called after a tool is executed."""
        pass

    async def on_error(self, message: str):
        """Called when an error occurs during the loop."""
        pass

    async def on_turn_complete(self, full_response: str, tool_calls: List[Dict[str, Any]]):
        """Called after a single LLM turn (response + tool results) is complete."""
        pass

    async def should_stop(self) -> bool:
        """Called to check if the loop should be terminated early."""
        return False

class AgentOrchestrator:
    """Unified execution loop for AI agents in LoomCLI."""
    def __init__(self, ctx: LoomContext):
        self.ctx = ctx
        self.provider = None
        self._initialize_provider()

    def _initialize_provider(self):
        api_key = self.ctx.config.get_api_key()
        if not api_key:
            return False
        
        provider_class = PROVIDER_MAP.get(self.ctx.config.provider)
        if provider_class:
            self.provider = provider_class(api_key)
            return True
        return False

    def microcompact(self, messages: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """
        Strip old tool results without an API call while preserving critical context.
        """
        if len(messages) < 6:
            return messages
            
        system_msg = messages[0]
        
        turns = []
        current_turn = []
        last_role = None
        for msg in messages[1:]:
            role = msg.get("role")
            if role == "user":
                if current_turn: turns.append(current_turn)
                current_turn = [msg]
            elif role == "assistant":
                if last_role in ("tool", "assistant"):
                    if current_turn: turns.append(current_turn)
                    current_turn = [msg]
                else:
                    current_turn.append(msg)
            else:
                current_turn.append(msg)
            last_role = role
            
        if current_turn:
            turns.append(current_turn)

        kept = [system_msg]
        for i, turn in enumerate(turns):
            is_recent = i >= len(turns) - 4
            has_critical_context = False
            for msg in turn:
                # Tools that produce context crucial for subsequent steps
                if msg.get("role") == "tool" and msg.get("name") in ("file_read", "file_edit", "file_write", "task"):
                    has_critical_context = True
                    break
                    
            if is_recent or has_critical_context:
                kept.extend(turn)

        return kept

    async def run(self, messages: List[Dict[str, Any]], hooks: Optional[OrchestratorHooks] = None, max_turns: int = 20, tool_executor: Optional[Callable] = None):
        """
        Runs the core agent loop: LLM call -> Tool execution -> State update.
        """
        if not self.provider:
            if not self._initialize_provider():
                if hooks: await hooks.on_error("Provider not initialized. Missing API key.")
                return

        hooks = hooks or OrchestratorHooks()
        tool_executor = tool_executor or self._call_tool_safe
        tool_schemas = [tool.to_json_schema() for tool in registry._tools.values() if tool.name != "task"]

        turn_count = 0
        while turn_count < max_turns:
            if await hooks.should_stop():
                break

            # Automatic context management
            usage_pct = self.ctx.state.get_context_usage(self.ctx.config.model)
            if usage_pct > 85:
                # Perform micro-compaction
                original_len = len(messages)
                compacted = self.microcompact(messages)
                if len(compacted) < original_len:
                    messages[:] = compacted
                    self.ctx.state.tokens_used = 0 # Reset to force recalculation or at least avoid false alarms
                    self.ctx.state.reset_context_warning()
                    # We don't have a specific hook for compaction yet, but we could add one
                    # For now, it just happens transparently

            turn_count += 1
            full_response = ""
            tool_calls = []
            start_time = time.time()

            try:
                provider_usage = None
                async for chunk in self.provider.ask(messages, self.ctx.config.model, tools=tool_schemas):
                    if await hooks.should_stop():
                        return
                    
                    await hooks.on_chunk(chunk)
                    
                    if chunk["type"] == "text":
                        full_response += chunk["content"]
                    elif chunk["type"] == "tool_call":
                        tool_calls.append(chunk["tool_call"])
                    elif chunk["type"] == "usage":
                        provider_usage = chunk["usage"]
                    elif chunk["type"] == "error":
                        await hooks.on_error(chunk["content"])
                        return

                # Record the assistant's message
                assistant_message = {"role": "assistant"}
                if full_response:
                    assistant_message["content"] = full_response
                if tool_calls:
                    sanitized = []
                    for tc in tool_calls:
                        t = dict(tc)
                        fn = dict(t.get("function", {}))
                        if isinstance(fn.get("arguments"), dict):
                            fn["arguments"] = json.dumps(fn["arguments"])
                        t["function"] = fn
                        sanitized.append(t)
                    assistant_message["tool_calls"] = sanitized
                
                messages.append(assistant_message)
                
                if provider_usage:
                    comp_tokens = provider_usage.get("completion_tokens", 0)
                    prompt_tokens = provider_usage.get("prompt_tokens", 0)
                    self.ctx.state.add_tokens(0, self.ctx.config.model, input_tokens=prompt_tokens, output_tokens=comp_tokens)
                elif full_response:
                    self.ctx.state.add_tokens(count_tokens(full_response, self.ctx.config.model), self.ctx.config.model)

                await hooks.on_turn_complete(full_response, tool_calls)

                if not tool_calls:
                    break

                # Tool execution logic
                tool_inputs = []
                for tc in tool_calls:
                    tc_id = tc.get("id")
                    func = tc.get("function", {})
                    name = func.get("name")
                    if not tc_id or not name: continue

                    raw_args = func.get("arguments", "{}")
                    try:
                        args = registry.parse_and_validate(name, raw_args)
                    except ValueError as e:
                        # If validation fails, we still want to record the tool result with an error
                        await self._append_tool_result(messages, tc_id, name, {"error": str(e)})
                        continue

                    tool_inputs.append((tc_id, name, args))

                # Partition into concurrent-safe batches
                batches = self._partition_tools(tool_inputs)
                for is_safe, items in batches:
                    if is_safe and len(items) > 1:
                        # Concurrent execution using asyncio.gather
                        tasks = []
                        for tc_id, name, args in items:
                            await hooks.on_tool_call(name, args)
                            # We wrap the tool_executor in a coroutine
                            async def _exec(tid=tc_id, n=name, a=args):
                                res = await tool_executor(n, a)
                                return tid, n, res

                            tasks.append(_exec())
                        
                        results = await asyncio.gather(*tasks)
                        for tid, n, res in results:
                            await self._append_tool_result(messages, tid, n, res)
                            await hooks.on_tool_result(n, res, 0.0)
                    else:
                        # Sequential execution
                        for tc_id, name, args in items:
                            await hooks.on_tool_call(name, args)
                            ts = time.time()
                            res = await tool_executor(name, args)
                            elapsed = time.time() - ts
                            await self._append_tool_result(messages, tc_id, name, res)
                            await hooks.on_tool_result(name, res, elapsed)

            except Exception as e:
                await hooks.on_error(f"Orchestrator error: {str(e)}")
                break

    def _partition_tools(self, tool_inputs: list) -> list:
        batches = []
        current_batch = []
        for item in tool_inputs:
            name = item[1]
            tool = registry.get_tool(name)
            is_safe = tool.isConcurrencySafe if tool else False
            if is_safe:
                current_batch.append(item)
            else:
                if current_batch:
                    batches.append((True, current_batch))
                    current_batch = []
                batches.append((False, [item]))
        if current_batch:
            batches.append((True, current_batch))
        return batches

    async def _call_tool_safe(self, name: str, args: dict) -> Dict[str, Any]:
        import asyncio
        tool = registry.get_tool(name)
        if not tool:
            return {"error": f"Tool not found: {name}"}
        try:
            # Most tools are still synchronous, so we run them in a thread to avoid blocking the event loop.
            return await asyncio.to_thread(tool.execute, **args, ctx=self.ctx)
        except Exception as e:
            return {"error": str(e)}

    async def _append_tool_result(self, messages: List[dict], tc_id: str, name: str, result: Dict[str, Any]):
        from .config import CONFIG_DIR
        from .storage import AtomicJsonStore
        import asyncio
        
        MAX_CHARS = 50000
        content = json.dumps(result)
        
        if len(content) > MAX_CHARS:
            path = CONFIG_DIR / "tool_results" / f"{tc_id}.json"
            store = AtomicJsonStore(path)
            await store.save_async(result)
            result["content"] = f"[Result too large, saved to {path}]\n{result.get('content', '')[:2000]}"
            content = json.dumps(result)

        messages.append({
            "role": "tool", 
            "tool_call_id": tc_id,
            "name": name, 
            "content": content
        })
        self.ctx.state.add_tokens(count_tokens(content, self.ctx.config.model), self.ctx.config.model)
