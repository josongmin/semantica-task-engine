#!/bin/bash
# Semantica Task Engine - Quick Start Script
# 다른 프로젝트에 빠르게 통합하기 위한 스크립트

set -e

echo "🚀 Semantica Task Engine - Quick Integration"
echo "==========================================="

# 1. 이미지 존재 확인
if ! docker images | grep -q "semantica-task-engine"; then
    echo "⚠️  Semantica Docker 이미지가 없습니다."
    echo "빌드하시겠습니까? (y/n)"
    read -r response
    if [[ "$response" =~ ^[Yy]$ ]]; then
        SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
        SEMANTICA_ROOT="$SCRIPT_DIR/../.."
        
        echo "📦 Building Semantica Docker image..."
        docker build -f "$SEMANTICA_ROOT/Dockerfile.dev" \
                     -t semantica-task-engine:latest \
                     "$SEMANTICA_ROOT"
        echo "✅ Build complete!"
    else
        echo "❌ 이미지가 필요합니다. 종료합니다."
        exit 1
    fi
fi

# 2. 타겟 프로젝트 경로
echo ""
echo "통합할 프로젝트 경로를 입력하세요:"
read -r TARGET_PROJECT

if [ ! -d "$TARGET_PROJECT" ]; then
    echo "❌ 경로가 존재하지 않습니다: $TARGET_PROJECT"
    exit 1
fi

cd "$TARGET_PROJECT"

# 3. SDK 복사
echo ""
echo "📋 Python SDK 복사 중..."
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cp "$SCRIPT_DIR/../python/semantica_client.py" ./
echo "✅ semantica_client.py 복사 완료"

# 4. docker-compose.yml 생성/수정
if [ -f "docker-compose.yml" ]; then
    echo "⚠️  docker-compose.yml 이 이미 존재합니다."
    echo "semantica-compose.yml 로 별도 생성하시겠습니까? (y/n)"
    read -r response
    if [[ "$response" =~ ^[Yy]$ ]]; then
        COMPOSE_FILE="semantica-compose.yml"
    else
        echo "직접 docker-compose.yml 에 추가하세요. (examples/integration/docker-compose.example.yml 참고)"
        exit 0
    fi
else
    COMPOSE_FILE="docker-compose.yml"
fi

cat > "$COMPOSE_FILE" <<'EOF'
version: '3.8'

services:
  semantica:
    image: semantica-task-engine:latest
    container_name: semantica-daemon
    ports:
      - "9527:9527"
    volumes:
      - semantica-data:/var/lib/semantica
    environment:
      - RUST_LOG=info
    healthcheck:
      test: ["CMD-SHELL", "curl -f http://localhost:9527/health || exit 1"]
      interval: 10s
      timeout: 5s
      retries: 3

volumes:
  semantica-data:
EOF

echo "✅ $COMPOSE_FILE 생성 완료"

# 5. requirements.txt 확인/추가
if [ -f "requirements.txt" ]; then
    if ! grep -q "requests" requirements.txt; then
        echo "requests>=2.31.0" >> requirements.txt
        echo "✅ requirements.txt에 requests 추가"
    fi
else
    echo "requests>=2.31.0" > requirements.txt
    echo "✅ requirements.txt 생성"
fi

# 6. 테스트 스크립트 생성
cat > test_semantica.py <<'EOF'
from semantica_client import SemanticaTaskClient, SemanticaError

def test_connection():
    print("🔍 Testing Semantica connection...")
    try:
        client = SemanticaTaskClient("http://localhost:9527")
        stats = client.stats()
        print(f"✅ Connected! Stats: {stats}")
        return True
    except SemanticaError as e:
        print(f"❌ Failed: {e}")
        return False

if __name__ == "__main__":
    test_connection()
EOF

echo "✅ test_semantica.py 생성"

# 완료
echo ""
echo "==========================================="
echo "✅ 통합 완료!"
echo ""
echo "다음 단계:"
echo "1. Semantica 실행:"
echo "   docker-compose -f $COMPOSE_FILE up -d"
echo ""
echo "2. 연결 테스트:"
echo "   python test_semantica.py"
echo ""
echo "3. 코드에서 사용:"
echo "   from semantica_client import SemanticaTaskClient"
echo "   client = SemanticaTaskClient()"
echo ""
echo "자세한 내용: examples/integration/README.md"


