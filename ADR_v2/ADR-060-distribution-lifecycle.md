This is the final ADR-060. It defines how the software is packaged, signed, delivered, and updated on user machines.

ADR-060: Distribution, Signing & Lifecycle
Status: Accepted

Date: 2024-XX-XX

Scope: CI/CD, Release Engineering, Client-side Update Logic

Tags: #distribution, #release, #signing, #update, #rust

1. Context
The Semantica Orchestrator is a compiled Rust binary. While Rust ensures memory safety, the delivery of the binary to user machines introduces external challenges:

OS Security Gates: macOS (Gatekeeper) and Windows (SmartScreen) actively block unsigned or unnotarized binaries, treating them as malware.

Update Friction: Users rarely update developer tools manually. To ship bug fixes and model improvements rapidly, the system requires a low-friction Self-Update mechanism.

Dependency Hell: We cannot rely on system libraries (glibc versions, openssl). The distribution must be self-contained (static linking where possible).

This ADR defines a SOTA Supply Chain using modern Rust tooling (cargo-dist, axoupdater) to solve these problems.

2. Supply Chain & Packaging Strategy
We adopt GitHub Releases as the primary distribution channel, orchestrated by cargo-dist.

2.1. Tooling Standard
Build Orchestrator: cargo-dist (The current standard for Rust binaries).

Role: Manages cross-compilation, archive generation (.tar.gz, .zip), and installer script creation (install.sh, install.ps1).

CI Environment: GitHub Actions.

Artifacts:

x86_64-unknown-linux-musl (Statically linked for Linux).

x86_64-apple-darwin (Intel Mac).

aarch64-apple-darwin (Apple Silicon).

x86_64-pc-windows-msvc (Windows).

2.2. Installation Scope
Decision: User-Scope Only.

Rationale: The Orchestrator is a developer tool. Requiring sudo or Admin privileges increases the security attack surface and friction.

Paths:

Linux/macOS: ~/.local/bin or ~/.semantica/bin (Added to $PATH).

Windows: %LOCALAPPDATA%\semantica\bin.

3. Code Signing & Notarization (The Trust Layer)
To prevent OS warnings ("This file is from an unidentified developer"), strict signing is enforced.

3.1. macOS (Gatekeeper)
Requirement: Apple Developer ID Application Certificate.

Process:

Sign: Codesign the binary with the certificate.

Notarize: Submit the zip/dmg to Apple's Notary Service via xcrun notarytool.

Staple: Attach the notary ticket to the executable.

Enforcement: CI fails if notarization fails.

3.2. Windows (SmartScreen)
Requirement: EV (Extended Validation) or OV Code Signing Certificate.

Process: Sign the .exe using signtool or Azure Key Vault during the CI process.

Effect: Prevents the "Windows protected your PC" popup which destroys user trust.

3.3. Linux
Requirement: GPG Signature.

Process: Generate SHA256SUMS and sign it with the project's GPG key.

4. Update Mechanism (Self-Update)
We implement a Pull-based Self-Update model.

4.1. Update Logic (Client-Side)
We use the axoupdater crate (or a custom implementation compliant with cargo-dist manifests).

Workflow:

Check: On startup (or via semantica update command), the CLI queries the GitHub Releases API (or a proxy).

Compare: Checks if Remote-SemVer > Local-SemVer.

Download: Fetches the appropriate pre-built asset for the current OS/Arch.

Swap:

Downloads new binary to semantica.new.

Renames current semantica to semantica.old (Windows safe move).

Renames semantica.new to semantica.

Restart: The daemon restarts gracefully to load the new version.

4.2. Update Channels
To support beta testing of AI features:

Stable: Default channel. Only receives tagged releases (e.g., v1.0.0).

Nightly: Builds from the main branch. Versioned as v0.0.0-nightly.YYYYMMDD. Users opt-in via config.

5. Versioning Policy
We strictly follow Semantic Versioning 2.0.0.

Major (X.y.z): Breaking changes to the JSON-RPC Contract (ADR-020) or Database Schema (ADR-010) that cannot be auto-migrated.

Minor (x.Y.z): New features, new Job Types, backward-compatible schema additions.

Patch (x.y.Z): Bug fixes, performance improvements, internal refactoring.

6. Implementation Plan
6.1. CI/CD Pipeline (GitHub Actions)
Test: Run Unit/Integration/Golden tests (ADR-030).

Audit: Run cargo audit (Security) and cargo deny (License check).

Build: Cross-compile for all targets using cargo-dist.

Sign: Inject secrets, Sign, and Notarize artifacts.

Publish: Upload to GitHub Releases.

6.2. Bootstrapping
A simple shell script (curl https://semantica.dev/install.sh | sh) will be provided as the initial entry point, which essentially downloads the latest binary and places it in the User Path.