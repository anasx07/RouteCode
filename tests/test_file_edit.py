import pytest
from dataclasses import dataclass
from routecode.tools.file_edit import FileEditTool
from routecode.core.path_guard import PathGuard


@dataclass
class MockContext:
    path_guard: PathGuard


@pytest.fixture
def ctx(tmp_path):
    return MockContext(path_guard=PathGuard(root=str(tmp_path)))


def test_file_edit_success(tmp_path, ctx):
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World\nThis is a test.")

    tool = FileEditTool()
    result = tool.execute("test.txt", "World", "RouteCode", ctx=ctx)

    assert result["success"] is True
    assert "Replaced 1 occurrence" in result["message"]
    assert test_file.read_text() == "Hello RouteCode\nThis is a test."


def test_file_edit_multiple_fail(tmp_path, ctx):
    test_file = tmp_path / "test.txt"
    test_file.write_text("test test test")

    tool = FileEditTool()
    result = tool.execute("test.txt", "test", "passed", ctx=ctx)

    assert result["success"] is False
    assert "found 3 times" in result["error"]


def test_file_edit_multiple_allow(tmp_path, ctx):
    test_file = tmp_path / "test.txt"
    test_file.write_text("test test test")

    tool = FileEditTool()
    result = tool.execute("test.txt", "test", "passed", allow_multiple=True, ctx=ctx)

    assert result["success"] is True
    assert "Replaced 3 occurrence(s)" in result["message"]
    assert test_file.read_text() == "passed passed passed"


def test_file_edit_not_found(ctx):
    tool = FileEditTool()
    result = tool.execute("non_existent.txt", "old", "new", ctx=ctx)

    assert result["success"] is False
    assert "File not found" in result["error"]
