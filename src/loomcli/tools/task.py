import json
import threading
from typing import Any, Dict, Optional
from pydantic import BaseModel, Field
from .base import BaseTool, registry
from ..config import config
from ..state import count_tokens
from ..task_manager import task_manager


class TaskInput(BaseModel):
    task: str = Field(..., description="The task description for the sub-agent to complete")
    max_iterations: int = Field(10, description="Maximum number of tool-call loops before returning")
    run_in_background: bool = Field(False, description="If True, run in background and return task_id immediately")


def _run_sub_agent(task: str, max_iterations: int, task_id: str, state):
    from ..agents.registry import PROVIDER_MAP

    api_key = config.get_api_key()
    if not api_key:
        task_manager.fail(task_id, "No API key configured")
        return

    provider_cls = PROVIDER_MAP.get(config.provider)
    if not provider_cls:
        task_manager.fail(task_id, f"Unknown provider: {config.provider}")
        return

    provider = provider_cls(api_key)

    messages = [
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
    ]

    tool_schemas = [t.to_json_schema() for t in registry._tools.values()
                    if t.name != "task"]

    output = ""
    iterations = 0

    while iterations < max_iterations:
        if task_manager.is_killed(task_id):
            output += "\n(Task was stopped)"
            task_manager.fail(task_id, "Task was killed by user")
            return

        iterations += 1
        full_response = ""
        tool_calls = []

        try:
            for chunk in provider.ask(messages, config.model, tools=tool_schemas):
                if task_manager.is_killed(task_id):
                    return
                if chunk["type"] == "text":
                    full_response += chunk["content"]
                elif chunk["type"] == "tool_call":
                    tool_calls.append(chunk["tool_call"])
                elif chunk["type"] == "error":
                    task_manager.fail(task_id, chunk["content"])
                    return

            clean = full_response
            if "<thought>" in clean and "</thought>" in clean:
                parts = clean.split("<thought>", 1)
                thought_parts = parts[1].split("</thought>", 1)
                clean = thought_parts[1].strip()

            if "<result>" in clean and "</result>" in clean:
                parts = clean.split("<result>", 1)
                inner = parts[1].split("</result>", 1)[0].strip()
                output += inner + "\n"
                task_manager.complete(task_id, {"success": True, "output": output})
                return

            if clean:
                output += clean + "\n"

            assistant_msg = {"role": "assistant"}
            if full_response:
                assistant_msg["content"] = full_response
            if tool_calls:
                assistant_msg["tool_calls"] = tool_calls
            messages.append(assistant_msg)

            if full_response:
                state.add_tokens(count_tokens(full_response, config.model), config.model)

            if not tool_calls:
                continue

            for tc in tool_calls:
                tc_id = tc.get("id")
                func = tc.get("function", {})
                name = func.get("name")
                if not tc_id or not name:
                    continue

                try:
                    args = json.loads(func.get("arguments", "{}")) if isinstance(func.get("arguments"), str) else func.get("arguments", {})
                except json.JSONDecodeError:
                    continue

                tool = registry.get_tool(name)
                if not tool:
                    tool_result = {"error": f"Tool not found: {name}"}
                else:
                    try:
                        tool_result = tool.execute(**args)
                    except Exception as e:
                        tool_result = {"error": str(e)}

                state.add_tokens(count_tokens(json.dumps(tool_result), config.model), config.model)

                messages.append({
                    "role": "tool",
                    "tool_call_id": tc_id,
                    "name": name,
                    "content": json.dumps(tool_result)
                })

        except Exception as e:
            output += f"\n(Error at iteration {iterations}: {e})"
            task_manager.fail(task_id, f"Sub-agent error at iteration {iterations}: {str(e)}")
            return

    output += "\n(Task completed with max iterations reached)"
    task_manager.complete(task_id, {"success": True, "output": output})


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

    def execute(self, task: str, max_iterations: int = 10, run_in_background: bool = False, state=None) -> Dict[str, Any]:
        from ..task_manager import generate_task_id
        task_id = generate_task_id()
        if state is None:
            from ..state import SessionState
            state = SessionState()

        if run_in_background:
            thread = threading.Thread(
                target=_run_sub_agent,
                args=(task, max_iterations, task_id, state),
                daemon=True
            )
            task_manager.create(task[:80], thread, task_id)
            thread.start()
            return {
                "success": True,
                "task_id": task_id,
                "status": "running",
                "message": f"Task {task_id} started in background."
            }

        task_manager.create(task[:80], None, task_id)
        _run_sub_agent(task, max_iterations, task_id, state)
        record = task_manager.get(task_id)
        if record and record.result:
            return record.result
        return {"success": False, "error": "Task failed silently"}
