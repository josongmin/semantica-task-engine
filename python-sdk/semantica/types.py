"""Semantica SDK Types"""

from dataclasses import dataclass
from typing import Any, Optional


@dataclass
class EnqueueRequest:
    """Job enqueue request"""

    job_type: str
    queue: str
    subject_key: str
    payload: Any
    priority: int = 0


@dataclass
class EnqueueResponse:
    """Job enqueue response"""

    job_id: str
    state: str
    queue: str


@dataclass
class CancelResponse:
    """Job cancel response"""

    job_id: str
    cancelled: bool


@dataclass
class TailLogsResponse:
    """Tail logs response"""

    job_id: str
    log_path: Optional[str]
    lines: list[str]

