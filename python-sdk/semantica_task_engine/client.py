"""Semantica Client Implementation"""

import httpx
from typing import Optional

from .errors import ConnectionError, RpcError
from .types import EnqueueRequest, EnqueueResponse, CancelResponse, TailLogsResponse
from .daemon import DaemonManager


class SemanticaTaskClient:
    """SemanticaTask Engine Client
    
    Example:
        >>> async with SemanticaTaskClient("http://127.0.0.1:9527") as client:
        ...     response = await client.enqueue(
        ...         EnqueueRequest(
        ...             job_type="INDEX_FILE",
        ...             queue="default",
        ...             subject_key="src/main.py",
        ...             payload={"path": "src/main.py"},
        ...         )
        ...     )
        ...     print(f"Job ID: {response.job_id}")
    """

    def __init__(
        self, 
        url: str = "http://127.0.0.1:9527", 
        timeout: float = 30.0,
        auto_start_daemon: bool = True
    ):
        """Initialize client
        
        Args:
            url: RPC endpoint URL
            timeout: Request timeout in seconds
            auto_start_daemon: Automatically start daemon if not running (default: True)
        """
        self.url = url
        self.timeout = timeout
        self.auto_start_daemon = auto_start_daemon
        self._client: Optional[httpx.AsyncClient] = None
        self._request_id = 0
        self._daemon_manager: Optional[DaemonManager] = None
        
        # Extract port from URL
        if ":" in url.split("//")[-1]:
            port = int(url.split(":")[-1].split("/")[0])
        else:
            port = 9527
        
        if auto_start_daemon:
            self._daemon_manager = DaemonManager(port=port, auto_start=True)

    async def __aenter__(self):
        """Context manager entry"""
        # Start daemon if needed
        if self._daemon_manager:
            await self._daemon_manager.start_daemon()
        
        self._client = httpx.AsyncClient(timeout=self.timeout)
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit"""
        if self._client:
            await self._client.aclose()
            self._client = None
        
        # Stop daemon if we started it
        if self._daemon_manager:
            await self._daemon_manager.stop_daemon()

    async def _request(self, method: str, params: dict) -> dict:
        """Send JSON-RPC request"""
        if not self._client:
            raise ConnectionError("Client not initialized. Use 'async with' context manager.")

        self._request_id += 1

        payload = {
            "jsonrpc": "2.0",
            "id": self._request_id,
            "method": method,
            "params": params,
        }

        try:
            response = await self._client.post(self.url, json=payload)
            response.raise_for_status()
        except httpx.HTTPError as e:
            raise ConnectionError(f"HTTP error: {e}") from e

        data = response.json()

        # Check for JSON-RPC error
        if "error" in data:
            error = data["error"]
            raise RpcError(
                code=error.get("code", -1),
                message=error.get("message", "Unknown error"),
                data=error.get("data"),
            )

        return data.get("result", {})

    async def enqueue(self, request: EnqueueRequest) -> EnqueueResponse:
        """Enqueue a new job
        
        Args:
            request: Job enqueue parameters
            
        Returns:
            EnqueueResponse with job_id, state, queue
            
        Example:
            >>> response = await client.enqueue(
            ...     EnqueueRequest(
            ...         job_type="INDEX_FILE",
            ...         queue="default",
            ...         subject_key="src/main.py",
            ...         payload={"path": "src/main.py"},
            ...         priority=5,
            ...     )
            ... )
            >>> print(response.job_id)
        """
        result = await self._request(
            "dev.enqueue.v1",
            {
                "job_type": request.job_type,
                "queue": request.queue,
                "subject_key": request.subject_key,
                "payload": request.payload,
                "priority": request.priority,
            },
        )

        return EnqueueResponse(
            job_id=result["job_id"],
            state=result["state"],
            queue=result["queue"],
        )

    async def cancel(self, job_id: str) -> CancelResponse:
        """Cancel a job
        
        Args:
            job_id: ID of the job to cancel
            
        Returns:
            CancelResponse with cancellation status
            
        Example:
            >>> response = await client.cancel("job-123")
            >>> if response.cancelled:
            ...     print("Job cancelled")
        """
        result = await self._request("dev.cancel.v1", {"job_id": job_id})

        return CancelResponse(
            job_id=result["job_id"],
            cancelled=result["cancelled"],
        )

    async def tail_logs(self, job_id: str, lines: int = 50) -> TailLogsResponse:
        """Tail job logs
        
        Args:
            job_id: ID of the job
            lines: Number of lines to retrieve (default: 50)
            
        Returns:
            TailLogsResponse with log lines
            
        Example:
            >>> response = await client.tail_logs("job-123", lines=100)
            >>> for line in response.lines:
            ...     print(line)
        """
        result = await self._request("logs.tail.v1", {"job_id": job_id, "lines": lines})

        return TailLogsResponse(
            job_id=result["job_id"],
            log_path=result.get("log_path"),
            lines=result.get("lines", []),
        )

