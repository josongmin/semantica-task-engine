#!/bin/bash
# Python SDK 배포 스크립트

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# 색상
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_info() {
    echo -e "${YELLOW}ℹ️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_header() {
    echo ""
    echo "================================"
    echo "$1"
    echo "================================"
}

# 버전 확인
VERSION=$(grep "^version" pyproject.toml | head -1 | cut -d'"' -f2)
print_header "Semantica Task SDK v$VERSION 배포"

# 1. 클린
print_info "이전 빌드 삭제 중..."
rm -rf dist/ build/ *.egg-info
print_success "클린 완료"

# 2. 테스트 (있으면 실행)
print_info "테스트 실행 중..."
if [ -d "tests" ] && [ "$(ls -A tests/*.py 2>/dev/null)" ]; then
    if pytest; then
        print_success "테스트 통과"
    else
        print_error "테스트 실패. 배포 중단."
        exit 1
    fi
else
    print_info "테스트 없음. 건너뜀."
fi

# 3. 빌드 도구 확인
print_info "빌드 도구 확인 중..."
pip install --upgrade build twine

# 4. 빌드
print_info "패키지 빌드 중..."
python -m build
print_success "빌드 완료"

# 5. 빌드 파일 확인
print_info "빌드 파일:"
ls -lh dist/

# 6. 배포 타겟 선택
echo ""
echo "배포 타겟 선택:"
echo "1) TestPyPI (테스트용)"
echo "2) PyPI (정식 배포)"
echo "3) 취소"
echo -n "선택 (1-3): "
read -r choice

case $choice in
    1)
        print_header "TestPyPI 업로드"
        print_info "TestPyPI에 업로드 중..."
        python -m twine upload --repository testpypi dist/*
        print_success "TestPyPI 업로드 완료!"
        echo ""
        print_info "설치 테스트:"
        echo "  pip install --index-url https://test.pypi.org/simple/ semantica-task-sdk==$VERSION"
        ;;
    2)
        print_header "PyPI 업로드"
        echo -n "정말로 PyPI에 v$VERSION을 배포하시겠습니까? (yes/no): "
        read -r confirm
        
        if [ "$confirm" == "yes" ]; then
            print_info "PyPI에 업로드 중..."
            python -m twine upload dist/*
            print_success "PyPI 업로드 완료!"
            echo ""
            print_info "설치 명령어:"
            echo "  pip install semantica-task-sdk==$VERSION"
            echo ""
            print_info "PyPI 페이지:"
            echo "  https://pypi.org/project/semantica-task-sdk/$VERSION/"
        else
            print_info "취소되었습니다."
        fi
        ;;
    3)
        print_info "취소되었습니다."
        exit 0
        ;;
    *)
        print_error "잘못된 선택입니다."
        exit 1
        ;;
esac

print_success "배포 스크립트 완료!"

