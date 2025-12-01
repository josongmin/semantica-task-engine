ADR-040: Security Policy & Sandboxing
Status: Accepted

Date: 2024-XX-XX

Scope: Global (IPC Auth, Subprocess Sandbox, Secret Management)

Tags: #security, #auth, #sandbox, #ipc

1. Context
The Semantica Orchestrator runs as a local daemon with high privileges (access to file system, network, and development tools). While it binds to local transports (UDS/Named Pipes), "Localhost" is not inherently safe.

Threat Model:

Malicious Local Processes: A compromised npm package, a malicious VSCode extension, or malware running under the same User ID could attempt to connect to the Orchestrator to execute arbitrary commands (dev.enqueue) or exfiltrate code.

Untrusted Subprocesses: A build script or test execution triggered by the Orchestrator (cargo test, npm install) could attempt to delete files or access internal Orchestrator state.

Secret Leakage: API keys or tokens passed to jobs could be leaked via logs or error messages.

This ADR defines a Zero Trust architecture even within the local machine boundary.

2. Layer 1: IPC Authentication (The Moat)
We implement a Dual-Layer Authentication mechanism for the JSON-RPC interface.

2.1. OS-Level Peer Verification
Before accepting a connection, the Daemon must verify the identity of the client process at the OS level.

Unix (UDS): Use SO_PEERCRED.

Rule: The Client's UID must match the Daemon's UID. Cross-user connections (e.g., from nobody or guest) are dropped immediately.

Windows (Named Pipes):

Rule: Apply strict DACLs (Discretionary Access Control Lists) to the Named Pipe, allowing only the Owner (Creator) and SYSTEM.

2.2. Application-Level Token (Bearer Auth)
Even if UID matches, we prevent unauthorized same-user processes (e.g., a random script) from connecting.

Mechanism:

Generation: On startup, the Daemon generates a high-entropy 256-bit random string (auth_token).

Storage: The token is written to ~/.semantica/connection.json with strict file permissions (0600 - read/write only by owner).

Handshake: Clients (SDK/CLI) must read this file and send the token in the initial request headers or connection handshake.

Enforcement:

If the token is missing or invalid, the connection is closed immediately.

Timing Attack Protection: Token comparison must be constant-time.

3. Layer 2: Execution Sandboxing (The Cage)
When execution_mode: SUBPROCESS is used, the Orchestrator acts as a supervisor. We must limit the blast radius of these child processes.

3.1. Environment Variable Sanitization
The Orchestrator's internal environment variables (containing its own config or DB paths) must NEVER leak to child processes.

Rule: Do not inherit the parent environment by default.

Allowlist: Explicitly pass only safe variables:

PATH (Sanitized)

HOME, USER, LANG

TERM

Project-specific variables defined in Job.env_vars.

3.2. Network Restrictions (Host Allowlist)
To prevent exfiltration of code by malicious build scripts:

Mechanism:

The Planner validates any url or host present in the Job payload against a strictly defined Host Allowlist.

Note: True network blocking (e.g., via unshare -n or Windows Firewall rules) is deferred to a future Hardening Phase. For Phase 1-4, we rely on Payload Validation.

Policy:

Allowed: github.com, crates.io, pypi.org, npmjs.com.

Blocked: Arbitrary IPs, suspicious domains.

3.3. Working Directory Confinement
Rule: All subprocesses must be spawned with current_dir set strictly within the user's Repository Root or a temporary Artifact directory.

Prevention: Block any job requesting a path traversal (e.g., ../, /etc/).

4. Layer 3: Data Privacy (Secret Management)
4.1. Secret Redaction (Logs)
Logs are persistent and dangerous.

Rule: No field named password, token, key, secret, or auth shall be logged in plain text.

Implementation:

Use a Redacted<T> wrapper type in Rust structs.

The Debug and Display implementations of Redacted<T> must output [REDACTED].

The tracing subscriber must be configured to respect these redactions.

4.2. Memory Hygiene
Secrets: API Keys injected into the Orchestrator (e.g., for LLM calls) should use the secrecy crate (or similar) to zero out memory on drop.

Persistence: Secrets must NEVER be stored in the jobs table (SQLite). They should be injected at runtime via the Bootstrap layer or strictly ephemeral env_vars.

5. Implementation Plan
5.1. Crate Selection
Auth: rand (Token gen), secrecy (Memory safety).

IPC: tokio (PeerCred support).

Permissions: std::os::unix::fs::PermissionsExt (File chmod).

5.2. Startup Sequence (Security Check)
Init: Generate auth_token.

File Write: Write connection.json -> chmod 600.

Socket Bind: Bind UDS/Pipe -> Check ownership permissions.

Listen: Accept loops start.