from abc import ABC, abstractmethod
from typing import Any, Callable, Dict, List, Optional, Type, TYPE_CHECKING
from pydantic import BaseModel

if TYPE_CHECKING:
    from ..core import LoomContext


class BaseTool(ABC):
    name: str
    description: str
    input_schema: Type[BaseModel]
    isConcurrencySafe: bool = False
    isReadOnly: bool = False
    isDestructive: bool = False

    def to_json_schema(self) -> Dict[str, Any]:
        schema = self.input_schema.model_json_schema()
        if "title" in schema:
            del schema["title"]
        for prop in schema.get("properties", {}).values():
            if "title" in prop:
                del prop["title"]
        return {
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": schema,
            },
        }

    def prompt(self) -> str:
        return f"- {self.name}: {self.description}"

    def get_activity_description(self, **kwargs) -> str:
        return self.name

    def get_tool_use_summary(self, **kwargs) -> str:
        return self.name

    @abstractmethod
    def execute(
        self,
        ctx: Optional["LoomContext"] = None,
        provider: Optional[Any] = None,
        **kwargs,
    ) -> Any:
        pass


HookFn = Callable[[str, Dict], None]


class ToolRegistry:
    def __init__(self):
        self._tools: Dict[str, BaseTool] = {}
        self._pre_hooks: List[HookFn] = []
        self._post_hooks: List[HookFn] = []

    def register(self, tool: BaseTool):
        self._tools[tool.name] = tool

    def get_tool(self, name: str) -> Optional[BaseTool]:
        return self._tools.get(name)

    def list_tools(self) -> Dict[str, str]:
        return {name: tool.description for name, tool in self._tools.items()}

    def list_tools_with_prompts(self) -> Dict[str, str]:
        return {name: tool.prompt() for name, tool in self._tools.items()}

    def add_pre_hook(self, fn: HookFn):
        self._pre_hooks.append(fn)

    def add_post_hook(self, fn: HookFn):
        self._post_hooks.append(fn)

    def run_pre_hooks(self, name: str, args: Dict):
        for fn in self._pre_hooks:
            try:
                fn(name, args)
            except Exception:
                pass

    def run_post_hooks(self, name: str, result: Dict):
        for fn in self._post_hooks:
            try:
                fn(name, result)
            except Exception:
                pass

    def parse_and_validate(self, name: str, arguments: Any) -> Dict[str, Any]:
        """
        Safely parses stringified JSON arguments and validates them against
        the tool's Pydantic model. Centralizes error handling and formatting.
        """
        import json
        from pydantic import ValidationError

        tool = self.get_tool(name)
        if not tool:
            raise ValueError(f"Unknown tool: {name}")

        # Parse JSON string if necessary
        if isinstance(arguments, str):
            try:
                args_dict = json.loads(arguments)
            except json.JSONDecodeError as e:
                raise ValueError(f"Invalid JSON in tool arguments: {str(e)}")
        elif isinstance(arguments, dict):
            args_dict = arguments
        else:
            raise ValueError(
                f"Arguments must be a JSON string or dictionary, got {type(arguments).__name__}"
            )

        # Validate with Pydantic
        try:
            validated = tool.input_schema.model_validate(args_dict)
            return validated.model_dump()
        except ValidationError as e:
            # Format pydantic errors into a more readable string
            errors = []
            for err in e.errors():
                loc = " -> ".join(str(part) for part in err["loc"])
                msg = err["msg"]
                errors.append(f" - {loc}: {msg}")
            raise ValueError(
                f"Validation failed for tool '{name}':\n" + "\n".join(errors)
            )


registry = ToolRegistry()
