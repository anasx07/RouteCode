"""
Typed stream chunks for the AI provider interface.

The provider's ask() method yields StreamChunk dicts representing
individual events in the streaming response.
"""

from typing import TypedDict, List, Dict, Any, Optional, Literal


class TextChunk(TypedDict):
    type: Literal["text"]
    content: str


class ThoughtChunk(TypedDict):
    type: Literal["thought"]
    content: str


class ToolCallChunk(TypedDict):
    type: Literal["tool_call"]
    tool_call: Dict[str, Any]


class UsageChunk(TypedDict):
    type: Literal["usage"]
    usage: Dict[str, Any]


class ErrorChunk(TypedDict):
    type: Literal["error"]
    content: str


StreamChunk = TextChunk | ThoughtChunk | ToolCallChunk | UsageChunk | ErrorChunk
"""Union of all possible stream chunk types yielded by ask()."""
