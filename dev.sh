#!/bin/bash
# Semantica Task Engine - Quick Development Script
# 빠르게 개발 환경 시작하기

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# 색상 정의
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 함수 정의
print_header() {
    echo -e "${BLUE}================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}================================${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_info() {
    echo -e "${YELLOW}ℹ️  $1${NC}"
}

# 메뉴 표시
show_menu() {
    print_header "Semantica Task Engine - Quick Dev"
    echo ""
    echo "1) 🚀 Daemon 시작 (빠른 실행)"
    echo "2) 🐛 Daemon 시작 (디버그 모드)"
    echo "3) 🐳 Docker로 시작"
    echo "4) 🐍 Python SDK 테스트"
    echo "5) 🧪 전체 테스트 실행"
    echo "6) 🗄️  DB 초기화"
    echo "7) 📊 현재 상태 확인"
    echo "8) 🛑 모든 Daemon 종료"
    echo "9) 📚 도움말"
    echo "0) 종료"
    echo ""
}

# Daemon 시작 (일반)
start_daemon() {
    print_header "Daemon 시작"
    
    # DB 디렉토리 생성
    mkdir -p ~/.semantica
    
    print_info "포트 9527에서 Daemon 시작 중..."
    RUST_LOG=info cargo run --package semantica-daemon
}

# Daemon 시작 (디버그)
start_daemon_debug() {
    print_header "Daemon 시작 (디버그)"
    
    mkdir -p ~/.semantica
    
    print_info "디버그 모드로 Daemon 시작 중..."
    RUST_LOG=debug cargo run --package semantica-daemon
}

# Docker 시작
start_docker() {
    print_header "Docker로 시작"
    
    if ! command -v docker-compose &> /dev/null; then
        print_error "docker-compose가 설치되어 있지 않습니다."
        return 1
    fi
    
    print_info "Docker 이미지 빌드 중..."
    docker build -f Dockerfile.dev -t semantica-task-engine:latest .
    
    print_info "Docker Compose 시작 중..."
    docker-compose -f docker-compose.dev.yml up
}

# Python SDK 테스트
test_python() {
    print_header "Python SDK 테스트"
    
    # Daemon 실행 여부 확인
    if ! lsof -i :9527 &> /dev/null; then
        print_error "Daemon이 실행 중이 아닙니다."
        print_info "먼저 Daemon을 시작하세요 (옵션 1 또는 2)"
        return 1
    fi
    
    print_info "Python SDK 설치 중..."
    cd python-sdk
    pip install -e . &> /dev/null || true
    
    print_info "Python 예제 실행 중..."
    cd examples
    python simple.py
    cd ../..
    
    print_success "Python SDK 테스트 완료!"
}

# 전체 테스트
run_tests() {
    print_header "전체 테스트 실행"
    
    print_info "Rust 테스트 실행 중..."
    cargo test
    
    print_success "모든 테스트 통과!"
}

# DB 초기화
reset_db() {
    print_header "DB 초기화"
    
    echo -n "정말로 DB를 초기화하시겠습니까? (y/N): "
    read -r confirm
    
    if [[ "$confirm" =~ ^[Yy]$ ]]; then
        rm -f ~/.semantica/meta.db*
        print_success "DB가 초기화되었습니다."
        print_info "Daemon을 재시작하여 DB를 다시 생성하세요."
    else
        print_info "취소되었습니다."
    fi
}

# 상태 확인
check_status() {
    print_header "현재 상태"
    
    echo ""
    echo "🔍 Daemon 상태:"
    if lsof -i :9527 &> /dev/null; then
        print_success "Daemon이 포트 9527에서 실행 중입니다."
        lsof -i :9527
    else
        print_error "Daemon이 실행 중이 아닙니다."
    fi
    
    echo ""
    echo "🗄️  DB 상태:"
    if [ -f ~/.semantica/meta.db ]; then
        print_success "DB 파일 존재: ~/.semantica/meta.db"
        db_size=$(du -h ~/.semantica/meta.db | cut -f1)
        echo "   크기: $db_size"
        
        job_count=$(sqlite3 ~/.semantica/meta.db "SELECT COUNT(*) FROM jobs;" 2>/dev/null || echo "0")
        echo "   Job 수: $job_count"
    else
        print_error "DB 파일이 존재하지 않습니다."
    fi
    
    echo ""
    echo "🐍 Python SDK:"
    if python3 -c "import semantica" 2>/dev/null; then
        print_success "Python SDK 설치됨"
    else
        print_error "Python SDK 미설치 (pip install -e python-sdk/)"
    fi
    
    echo ""
}

# 모든 Daemon 종료
kill_all() {
    print_header "모든 Daemon 종료"
    
    if pkill -f semantica-daemon; then
        print_success "Daemon이 종료되었습니다."
    else
        print_info "실행 중인 Daemon이 없습니다."
    fi
}

# 도움말
show_help() {
    print_header "도움말"
    
    echo ""
    echo "📚 빠른 명령어:"
    echo "  just start          - Daemon 시작"
    echo "  just start-debug    - 디버그 모드로 시작"
    echo "  just kill           - Daemon 종료"
    echo "  just restart        - Daemon 재시작"
    echo "  just py-example     - Python 예제 실행"
    echo "  just docker-dev     - Docker로 시작"
    echo "  just status         - 상태 확인"
    echo "  just db-reset       - DB 초기화"
    echo ""
    echo "📖 문서:"
    echo "  README.md                    - 프로젝트 소개"
    echo "  AI_ARCHITECTURE_GUIDE.md     - 전체 구조 가이드"
    echo "  python-sdk/README.md         - Python SDK 문서"
    echo "  python-sdk/QUICKSTART.md     - 5분 시작 가이드"
    echo ""
    echo "🔗 유용한 링크:"
    echo "  Daemon URL: http://localhost:9527"
    echo "  DB 경로: ~/.semantica/meta.db"
    echo ""
}

# 메인 루프
main() {
    while true; do
        show_menu
        echo -n "선택하세요 (0-9): "
        read -r choice
        
        case $choice in
            1)
                start_daemon
                ;;
            2)
                start_daemon_debug
                ;;
            3)
                start_docker
                ;;
            4)
                test_python
                ;;
            5)
                run_tests
                ;;
            6)
                reset_db
                ;;
            7)
                check_status
                ;;
            8)
                kill_all
                ;;
            9)
                show_help
                ;;
            0)
                print_info "종료합니다."
                exit 0
                ;;
            *)
                print_error "잘못된 선택입니다."
                ;;
        esac
        
        echo ""
        echo -n "계속하려면 Enter를 누르세요..."
        read -r
        clear
    done
}

# 스크립트 실행
if [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    show_help
    exit 0
fi

main

