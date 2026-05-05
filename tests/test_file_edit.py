import os
import pytest
from loomcli.tools.file_edit import FileEditTool

def test_file_edit_success(tmp_path):
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World\nThis is a test.")
    
    tool = FileEditTool()
    result = tool.execute(str(test_file), "World", "Loom")
    
    assert result["success"] is True
    assert "Replaced 1 occurrence" in result["message"]
    assert test_file.read_text() == "Hello Loom\nThis is a test."

def test_file_edit_multiple_fail(tmp_path):
    test_file = tmp_path / "test.txt"
    test_file.write_text("test test test")
    
    tool = FileEditTool()
    result = tool.execute(str(test_file), "test", "passed")
    
    assert result["success"] is False
    assert "found 3 times" in result["error"]

def test_file_edit_multiple_allow(tmp_path):
    test_file = tmp_path / "test.txt"
    test_file.write_text("test test test")
    
    tool = FileEditTool()
    result = tool.execute(str(test_file), "test", "passed", allow_multiple=True)
    
    assert result["success"] is True
    assert "Replaced 3 occurrence(s)" in result["message"]
    assert test_file.read_text() == "passed passed passed"

def test_file_edit_not_found(tmp_path):
    tool = FileEditTool()
    result = tool.execute("non_existent.txt", "old", "new")
    
    assert result["success"] is False
    assert "File not found" in result["error"]
