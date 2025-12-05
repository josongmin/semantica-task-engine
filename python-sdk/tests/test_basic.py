"""Basic tests for Semantica SDK"""

import pytest


def test_version():
    """Test version import"""
    from semantica_task_engine import __version__
    assert __version__ == "0.1.0"


def test_imports():
    """Test all public imports"""
    from semantica_task_engine import (
        SemanticaTaskClient,
        EnqueueRequest,
        EnqueueResponse,
        CancelResponse,
        TailLogsResponse,
        ConnectionError,
        RpcError,
    )
    
    # Check classes exist
    assert SemanticaTaskClient is not None
    assert EnqueueRequest is not None
    assert EnqueueResponse is not None
    assert CancelResponse is not None
    assert TailLogsResponse is not None
    assert ConnectionError is not None
    assert RpcError is not None


def test_enqueue_request_creation():
    """Test EnqueueRequest creation"""
    from semantica_task_engine import EnqueueRequest
    
    req = EnqueueRequest(
        job_type="TEST",
        queue="default",
        subject_key="test-1",
        payload={"msg": "hello"},
        priority=5
    )
    
    assert req.job_type == "TEST"
    assert req.queue == "default"
    assert req.subject_key == "test-1"
    assert req.payload == {"msg": "hello"}
    assert req.priority == 5


def test_enqueue_request_defaults():
    """Test EnqueueRequest default values"""
    from semantica_task_engine import EnqueueRequest
    
    req = EnqueueRequest(
        job_type="TEST",
        queue="default",
        subject_key="test-1",
        payload={}
    )
    
    assert req.priority == 0  # Default priority


def test_error_types():
    """Test error creation"""
    from semantica_task_engine.errors import ConnectionError, RpcError
    
    # ConnectionError
    conn_err = ConnectionError("Connection failed")
    assert str(conn_err) == "Connection failed"
    
    # RpcError
    rpc_err = RpcError(code=4001, message="Already exists")
    assert rpc_err.code == 4001
    assert rpc_err.message == "Already exists"
    assert rpc_err.data is None
    
    # RpcError with data
    rpc_err2 = RpcError(code=5000, message="Server error", data={"detail": "info"})
    assert rpc_err2.data == {"detail": "info"}

