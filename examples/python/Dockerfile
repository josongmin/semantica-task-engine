FROM python:3.11-slim

WORKDIR /app

# 의존성 설치
RUN pip install --no-cache-dir requests==2.31.0

# Python 클라이언트 복사
COPY semantica_client.py .
COPY test_integration.py .

# 기본 커맨드
CMD ["python", "test_integration.py"]


