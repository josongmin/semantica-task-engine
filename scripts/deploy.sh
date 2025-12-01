#!/usr/bin/env bash
# Semantica Task Engine - Deployment Script
# Phase 4: Production deployment automation

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() {
    echo -e "${CYAN}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

usage() {
    cat <<EOF
Usage: $0 <command> [options]

Commands:
    build       Build Docker image
    start       Start services
    stop        Stop services
    restart     Restart services
    logs        Tail logs
    status      Check service status
    test        Run deployment test
    clean       Clean up containers and volumes

Examples:
    $0 build
    $0 start
    $0 logs
    $0 test

EOF
    exit 1
}

cmd_build() {
    log_info "Building Docker image..."
    cd "$PROJECT_ROOT"
    docker compose build
    log_success "Build completed"
}

cmd_start() {
    log_info "Starting Semantica services..."
    cd "$PROJECT_ROOT"
    docker compose up -d
    log_success "Services started"
    
    log_info "Waiting for health check..."
    sleep 5
    docker compose ps
}

cmd_stop() {
    log_info "Stopping Semantica services..."
    cd "$PROJECT_ROOT"
    docker compose down
    log_success "Services stopped"
}

cmd_restart() {
    cmd_stop
    cmd_start
}

cmd_logs() {
    cd "$PROJECT_ROOT"
    docker compose logs -f semantica
}

cmd_status() {
    cd "$PROJECT_ROOT"
    log_info "Service status:"
    docker compose ps
    
    log_info ""
    log_info "Health check:"
    docker compose exec semantica semantica-cli status || true
}

cmd_test() {
    log_info "Running deployment test..."
    
    # 1. Build
    cmd_build
    
    # 2. Start
    cmd_start
    
    # 3. Wait for ready
    sleep 10
    
    # 4. Test enqueue
    log_info "Testing job enqueue..."
    docker compose exec semantica semantica-cli enqueue \
        --job-type TEST \
        --queue default \
        --subject "deploy-test" \
        --priority 0 \
        --payload '{"test": true}' || {
        log_error "Enqueue test failed"
        return 1
    }
    
    log_success "Deployment test passed"
}

cmd_clean() {
    log_info "Cleaning up Docker resources..."
    cd "$PROJECT_ROOT"
    docker compose down -v
    docker system prune -f
    log_success "Cleanup completed"
}

# Main
case "${1:-}" in
    build)
        cmd_build
        ;;
    start)
        cmd_start
        ;;
    stop)
        cmd_stop
        ;;
    restart)
        cmd_restart
        ;;
    logs)
        cmd_logs
        ;;
    status)
        cmd_status
        ;;
    test)
        cmd_test
        ;;
    clean)
        cmd_clean
        ;;
    *)
        usage
        ;;
esac

