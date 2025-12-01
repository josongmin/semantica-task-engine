This is the final ADR-020. It defines the API Contract, Error Model, and SDK Interfaces.ADR-020: API Contract & Error ModelStatus: AcceptedDate: 2024-XX-XXScope: JSON-RPC Interface (Northbound), SDKsTags: #api, #json-rpc, #sdk, #contract1. ContextThe Semantica Orchestrator exposes its core logic to external clients (IDEs, CLI, AI Agents) via a local IPC mechanism. To ensure consistent behavior across different clients (Python, TypeScript) and to support "Contract-First" development suitable for AI agents, we must rigorously define the Transport, Protocol, Error Semantics, and Method Signatures.This ADR serves as the Single Source of Truth for the API.2. Transport & ProtocolWe adhere strictly to JSON-RPC 2.0.2.1. Transport LayermacOS/Linux: Unix Domain Socket (UDS) at ~/.semantica/semantica.sock.Windows: Named Pipe at \\.\pipe\semantica-orchestrator.Fallback (Dev/Debug): TCP Loopback (e.g., 127.0.0.1:42069).2.2. JSON-RPC EnvelopeAll requests and responses must follow the standard envelope.Request:JSON{
  "jsonrpc": "2.0",
  "id": "req-123",
  "method": "dev.enqueue.v1",
  "params": { ... }
}
Response (Success):JSON{
  "jsonrpc": "2.0",
  "id": "req-123",
  "result": { ... }
}
Response (Error):JSON{
  "jsonrpc": "2.0",
  "id": "req-123",
  "error": {
    "code": 4000,
    "message": "Validation Error",
    "data": {
      "kind": "missing_field",
      "details": "field 'subject_key' is required",
      "trace_id": "trace-abc-123"
    }
  }
}
3. Error ModelWe distinguish between Client Errors (fixable by the caller) and System Errors (retryable or fatal).3.1. Error Code Ranges4000 ~ 4999: Client Errors (Validation, Logic, Not Found).5000 ~ 5999: Server Errors (Internal, IO, Panic).3.2. Standard Error CodesCodeMnemonicDescriptionAction4000VALIDATION_ERRORSchema violation, missing fields.Fix payload.4001NOT_FOUNDJob ID, Tag, or Group not found.Check ID.4002CONFLICTInvalid state transition, duplicate unique key.Resolve state.4003THROTTLEDRate limit or Backpressure active.Backoff & Retry.5000INTERNAL_ERRORUnexpected panic or logic bug.Report bug.5001DB_ERRORSQLite IO error, Corruption, Lock timeout.Check Disk/DB.5002SYSTEM_ERROROS Resource exhaustion (File handles, RAM).Free resources.4. API Specification4.1. Namespace: dev.* (Development Workflow)dev.enqueue.v1Enqueues a task for execution.Params:TypeScriptinterface EnqueueParams {
  job_type: string;          // e.g., "INDEX_FILE", "RUN_TEST"
  queue: string;             // e.g., "code_intel", "build"
  subject_key: string;       // Supersede Key (e.g., "repo::path")
  
  payload: any;              // JSON serializable object
  priority?: number;         // Default 0
  
  // Scheduling
  schedule?: {
    type: "IMMEDIATE" | "AT" | "AFTER" | "CONDITION";
    scheduled_at?: number;   // Epoch ms
    delay_ms?: number;
  };
  
  // Conditions (Phase 3+)
  conditions?: {
    wait_for_idle?: boolean;
    require_charging?: boolean;
    wait_for_event?: string; // e.g., "git_commit"
  };
  
  // Traceability & UX
  tag?: string;              // User-facing tag (e.g., "nightly")
  chain_group_id?: string;   // Workflow session ID
}
Result:TypeScriptinterface EnqueueResult {
  job_id: string;            // UUID v4
  queue: string;
  state: "QUEUED" | "SCHEDULED";
}
dev.cancel.v1Cancels jobs based on ID, Tag, or Group.Params:TypeScriptinterface CancelParams {
  job_id?: string;
  tag?: string;
  chain_group_id?: string;
}
// At least one field must be provided.
Result:TypeScriptinterface CancelResult {
  cancelled_count: number;
}
dev.query_jobs.v1Flexible search with cursor-based pagination.Params:TypeScriptinterface QueryParams {
  filter: {
    state?: JobState[];
    queue?: string[];
    tag?: string;
    chain_group_id?: string;
    subject_key_prefix?: string;
    created_after?: number;
  };
  sort?: "ASC" | "DESC";     // Default DESC
  limit?: number;            // Default 50, Max 200
  cursor?: string;           // Base64 opaque cursor
}
Result:TypeScriptinterface QueryResult {
  items: JobView[];
  next_cursor: string | null;
}
4.2. Namespace: logs.* (Observability)logs.tail.v1Streams log output for a job.Params:TypeScriptinterface TailParams {
  job_id: string;
  offset: number;            // Byte offset
  limit?: number;            // Max bytes to read
}
Result:TypeScriptinterface TailResult {
  chunk: string;             // UTF-8 string
  next_offset: number;
  eof: boolean;              // True if job is DONE and file end reached
}
4.3. Namespace: admin.* (Management)admin.stats.v1Retrieves system health and queue metrics.Params: {}Result:TypeScriptinterface StatsResult {
  queues: Array<{
    name: string;
    queued: number;
    running: number;
    failed: number;
    avg_wait_ms: number;
  }>;
  system: {
    cpu_usage: number;
    memory_usage: number;
    is_idle: boolean;
    db_wal_size: number;
  };
}
5. SDK Interface GuidelinesThe SDKs (Python, TypeScript) must provide a uniform, idiomatic wrapper around the JSON-RPC layer.5.1. General PrinciplesType Safety: Generate types from the Rust DTOs/Schemas where possible.Traceability: Auto-generate a trace_id for every request if one is not provided.Connection Management: Handle auto-reconnection logic transparently.5.2. Python SDK SignaturePythonclass SemanticaClient:
    def enqueue(self, job_type: str, queue: str, subject_key: str, payload: dict, **kwargs) -> str:
        """Returns job_id"""
        ...
    
    def cancel(self, tag: str = None, job_id: str = None) -> int:
        ...

    def tail_logs(self, job_id: str, follow: bool = True) -> Generator[str, None, None]:
        ...
5.3. TypeScript SDK SignatureTypeScriptclass SemanticaClient {
  enqueue(params: EnqueueParams): Promise<string>; // Returns job_id
  cancel(params: CancelParams): Promise<number>;
  query(params: QueryParams): Promise<QueryResult>;
}