"""Simple Python SDK Example

Usage:
    1. Start daemon: cargo run --package semantica-daemon
    2. Run example: python examples/simple.py
"""

import asyncio
from semantica import SematicaClient, EnqueueRequest


async def main():
    print("Semantica Python SDK - Simple Example")
    print("=" * 40)
    print()

    # 1. Connect to daemon
    print("1. Connecting to daemon...")
    async with SematicaClient("http://127.0.0.1:9527") as client:
        print("   ✓ Connected\n")

        # 2. Enqueue a job
        print("2. Enqueuing a job...")
        response = await client.enqueue(
            EnqueueRequest(
                job_type="INDEX_FILE",
                queue="default",
                subject_key="examples/simple.py",
                payload={"path": "examples/simple.py", "mode": "full_index"},
                priority=5,
            )
        )

        print("   ✓ Job enqueued:")
        print(f"     - ID: {response.job_id}")
        print(f"     - State: {response.state}")
        print(f"     - Queue: {response.queue}\n")

        # 3. Wait a bit
        print("3. Waiting 2 seconds...")
        await asyncio.sleep(2)
        print("   ✓ Done\n")

        # 4. Tail logs
        print("4. Fetching job logs...")
        logs_response = await client.tail_logs(response.job_id, lines=10)

        print("   ✓ Logs retrieved:")
        if logs_response.log_path:
            print(f"     - Path: {logs_response.log_path}")
        print(f"     - Lines: {len(logs_response.lines)}")

        if logs_response.lines:
            print(f"\n   Last {len(logs_response.lines)} lines:")
            for line in logs_response.lines:
                print(f"     | {line}")
        print()

        # 5. Cancel job
        print("5. Cancelling job...")
        cancel_response = await client.cancel(response.job_id)

        if cancel_response.cancelled:
            print("   ✓ Job cancelled")
        else:
            print("   ⚠ Job was already finished")

        print("\n✓ Example completed successfully!")


if __name__ == "__main__":
    asyncio.run(main())

