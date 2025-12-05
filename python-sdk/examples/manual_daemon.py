#!/usr/bin/env python3
"""
Semantica SDK - Manual Daemon Example

Daemon을 수동으로 관리하는 예제 (기존 방식)
"""

import asyncio
from semantica_task_engine import SemanticaTaskClient, EnqueueRequest


async def main():
    print("🚀 Semantica SDK - Manual Daemon Example")
    print("=" * 60)
    print("⚠️  Daemon must be running manually!")
    print("   Start with: just start")
    print()
    
    # auto_start_daemon=False
    # Daemon이 없으면 에러 발생
    try:
        async with SemanticaTaskClient(auto_start_daemon=False) as client:
            print("✅ Client connected to existing daemon")
            
            # Job 등록
            response = await client.enqueue(
                EnqueueRequest(
                    job_type="MANUAL_TEST",
                    queue="default",
                    subject_key="manual-test-1",
                    payload={"message": "Using existing daemon"},
                    priority=0
                )
            )
            
            print(f"\n✅ Job enqueued: {response.job_id}")
    
    except Exception as e:
        print(f"\n❌ Error: {e}")
        print("\n💡 Start daemon first:")
        print("   just start")
        print("   or")
        print("   cargo run --package semantica-daemon")


if __name__ == "__main__":
    asyncio.run(main())

