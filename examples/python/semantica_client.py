"""
Semantica Task Engine - Python SDK
JSON-RPC 2.0 클라이언트
"""
import os
import requests
from typing import Dict, Any, Optional, List


class SemanticaTaskClient:
    """Semantica Task Engine JSON-RPC 클라이언트"""
    
    def __init__(self, url: Optional[str] = None):
        """
        Args:
            url: RPC 엔드포인트 (기본값: http://127.0.0.1:9527)
        """
        self.url = url or os.getenv("SEMANTICA_RPC_URL", "http://127.0.0.1:9527")
        self.request_id = 0
        self.session = requests.Session()
    
    def _call(self, method: str, params: Dict[str, Any]) -> Dict[str, Any]:
        """JSON-RPC 호출"""
        self.request_id += 1
        payload = {
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": params
        }
        
        try:
            resp = self.session.post(self.url, json=payload, timeout=10)
            resp.raise_for_status()
            result = resp.json()
            
            if "error" in result:
                raise SemanticaRpcError(
                    code=result["error"]["code"],
                    message=result["error"]["message"],
                    data=result["error"].get("data")
                )
            
            return result["result"]
        
        except requests.exceptions.RequestException as e:
            raise SemanticaConnectionError(f"Connection failed: {e}")
    
    def enqueue(
        self,
        job_type: str,
        queue: str,
        subject_key: str,
        payload: Dict[str, Any],
        priority: int = 0
    ) -> Dict[str, Any]:
        """
        Job 등록
        
        Args:
            job_type: Job 타입 (예: "INDEX_FILE", "ANALYZE_CODE")
            queue: 큐 이름 (예: "default", "code_intel")
            subject_key: Subject key (파일 경로 등)
            payload: Job 데이터 (JSON 직렬화 가능한 dict)
            priority: 우선순위 (0=normal, 양수=높음, 음수=낮음)
        
        Returns:
            {"job_id": "uuid", "queue": "...", "state": "QUEUED"}
        """
        return self._call("dev.enqueue.v1", {
            "job_type": job_type,
            "queue": queue,
            "subject_key": subject_key,
            "payload": payload,
            "priority": priority
        })
    
    def cancel(self, job_id: str) -> Dict[str, Any]:
        """
        Job 취소
        
        Args:
            job_id: Job UUID
        
        Returns:
            {"job_id": "...", "state": "CANCELLED"}
        """
        return self._call("dev.cancel.v1", {"job_id": job_id})
    
    def tail_logs(self, job_id: str, limit: Optional[int] = 50) -> Dict[str, Any]:
        """
        로그 조회
        
        Args:
            job_id: Job UUID
            limit: 최대 라인 수 (기본값: 50)
        
        Returns:
            {"lines": ["log line 1", "log line 2", ...]}
        """
        params = {"job_id": job_id}
        if limit is not None:
            params["limit"] = limit
        
        return self._call("logs.tail.v1", params)
    
    def stats(self) -> Dict[str, Any]:
        """
        시스템 통계 조회
        
        Returns:
            {"queues": [{"name": "...", "queued": 10, ...}], ...}
        """
        return self._call("admin.stats.v1", {})


class SemanticaError(Exception):
    """Semantica SDK 베이스 에러"""
    pass


class SemanticaConnectionError(SemanticaError):
    """연결 실패 에러"""
    pass


class SemanticaRpcError(SemanticaError):
    """RPC 에러"""
    
    def __init__(self, code: int, message: str, data: Optional[Dict] = None):
        self.code = code
        self.message = message
        self.data = data
        super().__init__(f"RPC Error {code}: {message}")


