#!/usr/bin/env python3
"""
Semantica SDK - Auto Daemon Example

Daemon이 자동으로 시작/종료되는 예제
"""

import asyncio
from semantica_task_engine import SemanticaTaskClient, EnqueueRequest


async def main():
    print("🚀 Semantica SDK - Auto Daemon Example")
    print("=" * 60)
    
    # auto_start_daemon=True (기본값)
    # Daemon이 없으면 자동으로 시작함!
    async with SemanticaTaskClient(auto_start_daemon=True) as client:
        print("✅ Client connected (daemon auto-started if needed)")
        
        # Job 등록
        response = await client.enqueue(
            EnqueueRequest(
                job_type="AUTO_TEST",
                queue="default",
                subject_key="auto-test-1",
                payload={"message": "Daemon was auto-started!"},
                priority=0
            )
        )
        
        print(f"\n✅ Job enqueued:")
        print(f"   ID: {response.job_id}")
        print(f"   State: {response.state}")
        print(f"   Queue: {response.queue}")
        
        # 로그 조회
        await asyncio.sleep(1)  # Job 처리 대기
        
        logs = await client.tail_logs(response.job_id, lines=10)
        print(f"\n📋 Job logs:")
        for line in logs.lines:
            print(f"   {line}")
    
    print("\n✅ Example completed!")
    print("   (Daemon will be stopped automatically if we started it)")


if __name__ == "__main__":
    asyncio.run(main())

