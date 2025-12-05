# Justfile - Development Commands (ADR-002)
# Usage: just <command>

# Default task - show all commands
default:
    @just --list

# ============================================================================
# 🚀 Quick Start (가장 많이 사용)
# ============================================================================

# Start daemon (quick - 가장 빠른 실행)
start:
    @echo "🚀 Starting Semantica Daemon..."
    @mkdir -p ~/.semantica
    RUST_LOG=info cargo run --package semantica-daemon

# Start daemon (debug mode - 상세 로그)
start-debug:
    @echo "🐛 Starting Semantica Daemon (DEBUG mode)..."
    @mkdir -p ~/.semantica
    RUST_LOG=debug cargo run --package semantica-daemon

# Start daemon with custom port
start-port PORT:
    @echo "🚀 Starting Semantica Daemon on port {{PORT}}..."
    @mkdir -p ~/.semantica
    SEMANTICA_RPC_PORT={{PORT}} RUST_LOG=info cargo run --package semantica-daemon

# Kill all running daemons
kill:
    @echo "🛑 Killing all semantica processes..."
    @pkill -f semantica-daemon || echo "No daemon running"

# Restart daemon
restart: kill start

# ============================================================================
# 🐍 Python SDK
# ============================================================================

# Test Python SDK (daemon must be running)
py-test:
    @echo "🐍 Testing Python SDK..."
    cd python-sdk && python -m pytest -v

# Run Python example
py-example:
    @echo "🐍 Running Python example..."
    cd python-sdk/examples && python simple.py

# Install Python SDK
py-install:
    @echo "📦 Installing Python SDK..."
    cd python-sdk && pip install -e .

# ============================================================================
# 🐳 Docker
# ============================================================================

# Start daemon in Docker (development)
docker-dev:
    @echo "🐳 Starting Semantica in Docker (dev mode)..."
    docker-compose -f docker-compose.dev.yml up

# Start daemon in Docker (background)
docker-dev-bg:
    @echo "🐳 Starting Semantica in Docker (background)..."
    docker-compose -f docker-compose.dev.yml up -d

# Stop Docker daemon
docker-stop:
    @echo "🛑 Stopping Docker daemon..."
    docker-compose -f docker-compose.dev.yml down

# View Docker logs
docker-logs:
    docker-compose -f docker-compose.dev.yml logs -f

# Build Docker image
docker-build:
    @echo "🔨 Building Docker image..."
    docker build -f Dockerfile.dev -t semantica-task-engine:latest .

# ============================================================================
# 🧪 Testing
# ============================================================================

# Run all tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Run specific test
test-one TEST:
    cargo test {{TEST}} -- --nocapture

# Run integration tests
test-integration:
    cargo test --package integration-tests

# Run tests with coverage
test-coverage:
    cargo tarpaulin --out Html --output-dir coverage

# ============================================================================
# 🔨 Building
# ============================================================================

# Build (debug mode - 빠름)
build:
    cargo build

# Build (release mode - 최적화)
build-release:
    cargo build --release

# Build with telemetry
build-telemetry:
    cargo build --release --features telemetry

# Clean build artifacts
clean:
    cargo clean
    rm -rf target/

# ============================================================================
# 🎨 Code Quality
# ============================================================================

# Format code
fmt:
    cargo fmt

# Check formatting
fmt-check:
    cargo fmt -- --check

# Run clippy (lint)
lint:
    cargo clippy --all-targets -- -D warnings

# Fix clippy warnings automatically
fix:
    cargo clippy --fix --allow-dirty

# Full quality check (format + lint + test)
check: fmt-check lint test

# ============================================================================
# 📚 Documentation
# ============================================================================

# Generate and open documentation
docs:
    cargo doc --open --no-deps

# Generate docs for all dependencies
docs-all:
    cargo doc --open

# ============================================================================
# 🔧 Development Workflow
# ============================================================================

# Development loop (format + lint + test)
dev: fmt lint test

# Watch for changes and run tests
watch:
    cargo watch -x test

# Watch and run daemon
watch-daemon:
    cargo watch -x "run --package semantica-daemon"

# Quick iteration (build + run)
quick: build start

# ============================================================================
# 🗄️ Database
# ============================================================================

# Reset database
db-reset:
    @echo "🗄️ Resetting database..."
    rm -f ~/.semantica/meta.db*
    @echo "✅ Database reset. Start daemon to recreate."

# View database
db-view:
    sqlite3 ~/.semantica/meta.db

# Show recent jobs
db-jobs:
    @echo "📋 Recent jobs:"
    @sqlite3 ~/.semantica/meta.db "SELECT id, job_type, state, priority, created_at FROM jobs ORDER BY created_at DESC LIMIT 10;"

# ============================================================================
# 🚢 Deployment
# ============================================================================

# Build for production
prod-build:
    cargo build --release --features telemetry
    strip target/release/semantica

# Run production build
prod-run:
    SEMANTICA_LOG_FORMAT=json \
    SEMANTICA_DB_PATH=/var/lib/semantica/meta.db \
    ./target/release/semantica

# ============================================================================
# 🧹 Maintenance
# ============================================================================

# Update dependencies
update:
    cargo update

# Check for outdated dependencies
outdated:
    cargo outdated

# Security audit
audit:
    cargo audit

# Benchmark
bench:
    cargo bench

# ============================================================================
# 📊 Helpful Info
# ============================================================================

# Show current daemon status
status:
    @echo "🔍 Checking daemon status..."
    @lsof -i :9527 || echo "Daemon not running on port 9527"

# Show logs
logs:
    tail -f ~/.semantica/logs/*.log 2>/dev/null || echo "No logs found"

# Show environment
env:
    @echo "📊 Semantica Environment:"
    @echo "  DB Path: ~/.semantica/meta.db"
    @echo "  RPC Port: 9527 (default)"
    @echo "  Log Format: text (default)"
    @echo ""
    @echo "Override with:"
    @echo "  SEMANTICA_DB_PATH=<path>"
    @echo "  SEMANTICA_RPC_PORT=<port>"
    @echo "  SEMANTICA_LOG_FORMAT=json"

