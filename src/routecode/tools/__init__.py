from .base import registry
from .bash import BashTool
from .file_edit import FileEditTool
from .file_write import FileWriteTool
from .file_read import FileReadTool
from .glob import GlobTool
from .grep import GrepTool
from .task import TaskTool
from .skill import SkillTool, SkillCreatorTool, FindSkillsTool

from .webfetch import WebFetchTool
from .web_search import WebSearchTool
from .auth import AuthorizationMiddleware as AuthorizationMiddleware

registry.register(BashTool())
registry.register(FileEditTool())
registry.register(FileWriteTool())
registry.register(FileReadTool())
registry.register(GlobTool())
registry.register(GrepTool())
registry.register(TaskTool())
registry.register(SkillTool())
registry.register(SkillCreatorTool())
registry.register(FindSkillsTool())
registry.register(WebFetchTool())
registry.register(WebSearchTool())
