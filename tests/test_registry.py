from loomcli.tools.base import ToolRegistry, BaseTool
from pydantic import BaseModel


class MockInput(BaseModel):
    arg1: str


class MockTool(BaseTool):
    name = "mock_tool"
    description = "A mock tool for testing"
    input_schema = MockInput

    def execute(self, arg1: str):
        return f"Hello {arg1}"


def test_registry_register_and_get():
    registry = ToolRegistry()
    tool = MockTool()
    registry.register(tool)

    assert registry.get_tool("mock_tool") == tool
    assert registry.get_tool("non_existent") is None


def test_registry_list_tools():
    registry = ToolRegistry()
    tool = MockTool()
    registry.register(tool)

    tools = registry.list_tools()
    assert "mock_tool" in tools
    assert tools["mock_tool"] == "A mock tool for testing"


def test_tool_to_json_schema():
    tool = MockTool()
    schema = tool.to_json_schema()

    assert schema["type"] == "function"
    assert schema["function"]["name"] == "mock_tool"
    assert "arg1" in schema["function"]["parameters"]["properties"]
