import json
import random
import string
import threading
import time
from typing import Dict, Optional
from dataclasses import dataclass, field
from .config import config
from .events import bus


@dataclass
class TaskRecord:
    task_id: str
    description: str
    status: str  # pending | running | completed | failed | killed
    created_at: float = 0.0
    completed_at: float = 0.0
    result: Optional[dict] = None
    error: Optional[str] = None
    thread: Optional[threading.Thread] = None


def generate_task_id() -> str:
    prefix = random.choice(string.ascii_lowercase)
    suffix = ''.join(random.choices(string.ascii_lowercase + string.digits, k=7))
    return f"{prefix}{suffix}"


class TaskManager:
    def __init__(self):
        self._tasks: Dict[str, TaskRecord] = {}
        self._lock = threading.Lock()

    def create(self, description: str, thread: Optional[threading.Thread] = None, task_id: Optional[str] = None) -> str:
        if task_id is None:
            task_id = generate_task_id()
        with self._lock:
            self._tasks[task_id] = TaskRecord(
                task_id=task_id,
                description=description[:80],
                status="running",
                created_at=time.time(),
                thread=thread,
            )
            record = self._tasks[task_id]
        bus.emit("task.created", task_id=task_id, description=record.description)
        return task_id

    def complete(self, task_id: str, result: dict):
        with self._lock:
            if task_id in self._tasks:
                record = self._tasks[task_id]
                record.status = "completed"
                record.completed_at = time.time()
                record.result = result
                bus.emit("task.completed", task_id=task_id, description=record.description, result=result)

    def fail(self, task_id: str, error: str):
        with self._lock:
            if task_id in self._tasks:
                record = self._tasks[task_id]
                record.status = "failed"
                record.completed_at = time.time()
                record.error = error
                bus.emit("task.failed", task_id=task_id, description=record.description, error=error)

    def kill(self, task_id: str) -> bool:
        with self._lock:
            if task_id not in self._tasks:
                return False
            record = self._tasks[task_id]
            if record.status != "running":
                return False
            record.status = "killed"
            record.completed_at = time.time()
            # Thread will check status on next iteration
            return True

    def get(self, task_id: str) -> Optional[TaskRecord]:
        with self._lock:
            return self._tasks.get(task_id)

    def list(self) -> list:
        with self._lock:
            return sorted(
                [dict(
                    task_id=t.task_id,
                    description=t.description,
                    status=t.status,
                    created_at=t.created_at,
                    completed_at=t.completed_at,
                ) for t in self._tasks.values()],
                key=lambda x: x["created_at"],
                reverse=True
            )

    def is_killed(self, task_id: str) -> bool:
        with self._lock:
            record = self._tasks.get(task_id)
            return record is not None and record.status == "killed"


task_manager = TaskManager()
