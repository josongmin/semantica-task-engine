"""Daemon Auto-Management

Automatically start/stop Semantica Daemon when needed.
"""

import asyncio
import os
import subprocess
import time
from typing import Optional
import httpx


class DaemonManager:
    """Automatically manage Semantica Daemon lifecycle"""
    
    def __init__(self, port: int = 9527, auto_start: bool = True):
        """
        Args:
            port: Daemon port (default: 9527)
            auto_start: Automatically start daemon if not running (default: True)
        """
        self.port = port
        self.auto_start = auto_start
        self.process: Optional[subprocess.Popen] = None
        self._started_by_us = False
    
    async def is_daemon_running(self) -> bool:
        """Check if daemon is already running"""
        try:
            async with httpx.AsyncClient(timeout=2.0) as client:
                response = await client.post(
                    f"http://localhost:{self.port}",
                    json={
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "admin.stats.v1",
                        "params": {}
                    }
                )
                return response.status_code == 200
        except (httpx.ConnectError, httpx.TimeoutException):
            return False
    
    async def start_daemon(self) -> bool:
        """Start daemon if not running
        
        Returns:
            True if daemon was started, False if already running
        """
        # Check if already running
        if await self.is_daemon_running():
            return False
        
        if not self.auto_start:
            raise RuntimeError(
                f"Daemon is not running on port {self.port}. "
                f"Start it manually or set auto_start=True"
            )
        
        # Find daemon binary
        daemon_path = self._find_daemon_binary()
        if not daemon_path:
            raise RuntimeError(
                "Semantica daemon binary not found. "
                "Install it with: pip install semantica-task-sdk[daemon] "
                "or run manually: cargo run --package semantica-daemon"
            )
        
        # Start daemon
        env = os.environ.copy()
        env["SEMANTICA_RPC_PORT"] = str(self.port)
        env["RUST_LOG"] = env.get("RUST_LOG", "info")
        
        # Create data directory
        data_dir = os.path.expanduser("~/.semantica")
        os.makedirs(data_dir, exist_ok=True)
        
        self.process = subprocess.Popen(
            [daemon_path],
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True  # Detach from parent
        )
        
        self._started_by_us = True
        
        # Wait for daemon to be ready
        for _ in range(30):  # 30 seconds timeout
            await asyncio.sleep(0.5)
            if await self.is_daemon_running():
                return True
        
        raise RuntimeError(
            f"Daemon failed to start within 30 seconds. "
            f"Check logs at ~/.semantica/logs/"
        )
    
    async def stop_daemon(self):
        """Stop daemon if we started it"""
        if self.process and self._started_by_us:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
            self.process = None
            self._started_by_us = False
    
    def _find_daemon_binary(self) -> Optional[str]:
        """Find semantica daemon binary
        
        Search order:
        1. Environment variable SEMANTICA_DAEMON_PATH
        2. In PATH (semantica or semantica-daemon)
        3. In target/release/ (development)
        4. In target/debug/ (development)
        """
        # 1. Environment variable
        if path := os.getenv("SEMANTICA_DAEMON_PATH"):
            if os.path.isfile(path) and os.access(path, os.X_OK):
                return path
        
        # 2. In PATH
        for name in ["semantica", "semantica-daemon"]:
            try:
                result = subprocess.run(
                    ["which", name],
                    capture_output=True,
                    text=True,
                    timeout=1
                )
                if result.returncode == 0 and result.stdout.strip():
                    return result.stdout.strip()
            except (subprocess.TimeoutExpired, FileNotFoundError):
                pass
        
        # 3. Development build (release)
        dev_paths = [
            "target/release/semantica",
            "../target/release/semantica",
            "../../target/release/semantica",
        ]
        for path in dev_paths:
            abs_path = os.path.abspath(path)
            if os.path.isfile(abs_path) and os.access(abs_path, os.X_OK):
                return abs_path
        
        # 4. Development build (debug)
        debug_paths = [
            "target/debug/semantica",
            "../target/debug/semantica",
            "../../target/debug/semantica",
        ]
        for path in debug_paths:
            abs_path = os.path.abspath(path)
            if os.path.isfile(abs_path) and os.access(abs_path, os.X_OK):
                return abs_path
        
        return None

