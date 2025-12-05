"""Semantica Task SDK

A Python client for SemanticaTask Engine.
"""

__version__ = "0.1.0"

from .client import SemanticaTaskClient
from .types import (
    EnqueueRequest,
    EnqueueResponse,
    CancelResponse,
    TailLogsResponse,
)
from .errors import ConnectionError, RpcError

__all__ = [
    "SemanticaTaskClient",
    "EnqueueRequest",
    "EnqueueResponse",
    "CancelResponse",
    "TailLogsResponse",
    "ConnectionError",
    "RpcError",
    "__version__",
]
