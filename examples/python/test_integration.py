"""
Semantica Task Engine - Integration Test
Docker 환경에서 daemon 연동 테스트
"""
import time
from semantica_client import SemanticaTaskClient, SemanticaError


def main():
    print("🚀 Semantica Task Engine - Python Integration Test")
    print("=" * 60)
    
    client = SemanticaTaskClient()
    
    # 1. 연결 확인
    print("\n[1] Checking connection...")
    try:
        stats = client.stats()
        print(f"✅ Connected! Queues: {len(stats.get('queues', []))}")
    except SemanticaError as e:
        print(f"❌ Connection failed: {e}")
        return
    
    # 2. Job 등록
    print("\n[2] Enqueuing jobs...")
    jobs = []
    for i in range(3):
        try:
            resp = client.enqueue(
                job_type="TEST_JOB",
                queue="default",
                subject_key=f"test-file-{i}.py",
                payload={
                    "path": f"test-file-{i}.py",
                    "action": "index",
                    "test_id": i
                },
                priority=i
            )
            job_id = resp["job_id"]
            jobs.append(job_id)
            print(f"  ✅ Job {i+1}: {job_id[:8]}... (state: {resp['state']})")
        except SemanticaError as e:
            print(f"  ❌ Failed to enqueue job {i+1}: {e}")
    
    # 3. 상태 확인
    print("\n[3] Checking stats...")
    time.sleep(1)
    stats = client.stats()
    for queue in stats.get("queues", []):
        print(f"  Queue '{queue['name']}': "
              f"queued={queue.get('queued', 0)}, "
              f"running={queue.get('running', 0)}")
    
    # 4. 로그 확인
    if jobs:
        print(f"\n[4] Checking logs for job {jobs[0][:8]}...")
        try:
            logs = client.tail_logs(jobs[0], limit=20)
            lines = logs.get("lines", [])
            if lines:
                for line in lines[:5]:
                    print(f"  📝 {line}")
                if len(lines) > 5:
                    print(f"  ... ({len(lines) - 5} more lines)")
            else:
                print("  ℹ️  No logs yet")
        except SemanticaError as e:
            print(f"  ⚠️  Could not fetch logs: {e}")
    
    # 5. Job 취소
    if len(jobs) >= 2:
        print(f"\n[5] Cancelling job {jobs[1][:8]}...")
        try:
            resp = client.cancel(jobs[1])
            print(f"  ✅ Cancelled: {resp.get('state')}")
        except SemanticaError as e:
            print(f"  ❌ Cancel failed: {e}")
    
    print("\n" + "=" * 60)
    print("✅ Integration test completed!")


if __name__ == "__main__":
    main()


