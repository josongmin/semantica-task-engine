"""Semantica SDK Errors"""

from typing import Any, Optional


class SematicaError(Exception):
    """Base exception for Semantica SDK"""

    pass


class ConnectionError(SematicaError):
    """Connection error"""

    def __init__(self, message: str):
        self.message = message
        super().__init__(message)


class RpcError(SematicaError):
    """JSON-RPC error"""

    def __init__(self, code: int, message: str, data: Optional[Any] = None):
        self.code = code
        self.message = message
        self.data = data
        super().__init__(f"RPC error {code}: {message}")

