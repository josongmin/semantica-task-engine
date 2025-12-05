"""SemanticaTask SDK Errors"""

from typing import Any, Optional


class SemanticaTaskError(Exception):
    """Base exception for SemanticaTask SDK"""

    pass


class ConnectionError(SemanticaTaskError):
    """Connection error"""

    def __init__(self, message: str):
        self.message = message
        super().__init__(message)


class RpcError(SemanticaTaskError):
    """JSON-RPC error"""

    def __init__(self, code: int, message: str, data: Optional[Any] = None):
        self.code = code
        self.message = message
        self.data = data
        super().__init__(f"RPC error {code}: {message}")

