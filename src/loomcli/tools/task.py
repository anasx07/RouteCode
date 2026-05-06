import json
import threading
from typing import Any, Dict, Optional
from pydantic import BaseModel, Field
from .base import BaseTool, registry
from ..config import config
from ..core import count_tokens, SessionState, LoomContext
from ..orchestrator import AgentOrchestrator, OrchestratorHooks
from ..core import ConversationHistory
from ..utils import strip_thought, extract_tag


class TaskInput(BaseModel):
    task: str = Field(..., description="The task description for the sub-agent to complete")
    max_iterations: int = Field(10, description="Maximum number of tool-call loops before returning")
    run_in_background: bool = Field(False, description="If True, run in background and return task_id immediately")


async def _run_sub_agent_async(task: str, max_iterations: int, task_id: str, ctx: LoomContext, provider: Optional[Any] = None):
    orchestrator = AgentOrchestrator(ctx, provider=provider)
    if not orchestrator.provider:
        ctx.task_manager.fail(task_id, "No API key configured or provider unavailable")
        return

    history = ConversationHistory([
        {
            "role": "system",
            "content": (
                "You are a task-focused autonomous agent. Complete the assigned task "
                "by using the available tools. When you finish, provide a summary of what was done.\n\n"
                "IMPORTANT: After completing the task, respond with <result>your summary here</result> "
                "to signal that the task is complete."
            )
        },
        {"role": "user", "content": task}
    ])

    output = {"text": "", "completed": False}

    class TaskHooks(OrchestratorHooks):
        async def on_error(self, message):
            ctx.task_manager.fail(task_id, message)

        async def on_turn_complete(self, full_response, tool_calls):
            thought, clean = strip_thought(full_response)
            result = extract_tag(clean, "result")
            
            if result is not None:
                output["text"] += result + "\n"
                ctx.task_manager.complete(task_id, {"success": True, "output": output["text"]})
                output["completed"] = True
                return

            if clean:
                output["text"] += clean + "\n"

        async def should_stop(self) -> bool:
            if ctx.task_manager.is_killed(task_id):
                output["text"] += "\n(Task was stopped)"
                return True
            return output["completed"]

    await orchestrator.run(history, hooks=TaskHooks(), max_turns=max_iterations)

    if not output["completed"]:
        output["text"] += "\n(Task completed with max iterations reached)"
        ctx.task_manager.complete(task_id, {"success": True, "output": output["text"]})

def _run_sub_agent(task: str, max_iterations: int, task_id: str, ctx: LoomContext, provider: Optional[Any] = None):
    import asyncio
    asyncio.run(_run_sub_agent_async(task, max_iterations, task_id, ctx, provider=provider))



class TaskTool(BaseTool):
    name = "task"
    description = "Launch an autonomous sub-agent to complete a complex multi-step task. The sub-agent has access to all tools."
    input_schema = TaskInput

    def prompt(self) -> str:
        return ("- task: Delegate complex multi-step work to an autonomous sub-agent. "
                "Provide a clear task description. Use for tasks requiring many steps, "
                "background work, or isolated execution.")

    def get_activity_description(self, task: str = "", **kwargs) -> str:
        return f"Task({task[:50]})"

    def execute(self, task: str, max_iterations: int = 10, run_in_background: bool = False, 
                ctx: Optional[LoomContext] = None, provider: Optional[Any] = None, **kwargs) -> Dict[str, Any]:
        from ..task_manager import generate_task_id
        task_id = generate_task_id()
        if ctx is None:
            ctx = LoomContext(
                state=SessionState(),
                config=config,
                console=None, # Sub-agents don't usually have a console
                task_manager=None # Will be set below or use global
            )
            # This is a fallback for direct calls. 
            # In practice, ctx will always be passed from the REPL.

        if run_in_background:
            thread = threading.Thread(
                target=_run_sub_agent,
                args=(task, max_iterations, task_id, ctx, provider),
                daemon=True
            )
            ctx.task_manager.create(task[:80], thread, task_id)
            thread.start()
            return {
                "success": True,
                "task_id": task_id,
                "status": "running",
                "message": f"Task {task_id} started in background."
            }

        ctx.task_manager.create(task[:80], None, task_id)
        _run_sub_agent(task, max_iterations, task_id, ctx, provider=provider)
        record = ctx.task_manager.get(task_id)
        if record and record.result:
            return record.result
        return {"success": False, "error": "Task failed silently"}
