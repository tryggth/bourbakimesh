"""PyTest test suite for asynchronous Python Mesh IPC bridge client."""

import asyncio
import json
import uuid
import pytest
from bourbakimesh.ipc_client import AsyncMeshClient


@pytest.mark.asyncio
async def test_async_mesh_client_with_mock_server():
    """Verify AsyncMeshClient connects, transmits commands, and parses responses."""
    received_commands = []

    async def mock_handler(reader: asyncio.StreamReader, writer: asyncio.StreamWriter):
        while True:
            line = await reader.readline()
            if not line:
                break
            cmd = json.loads(line.decode("utf-8").strip())
            received_commands.append(cmd)

            if "Heartbeat" in cmd:
                resp = "Acknowledged"
            elif "ClaimTask" in cmd:
                task_data = cmd["ClaimTask"]
                resp = {
                    "TaskAssigned": {
                        "task_id": task_data["task_id"],
                        "goal_statement": task_data["goal_statement"],
                    }
                }
            elif "SubmitProof" in cmd:
                task_data = cmd["SubmitProof"]
                resp = {
                    "ProofAccepted": {
                        "task_id": task_data["task_id"],
                        "block_id": "0" * 64,
                    }
                }
            else:
                resp = "Pong"

            msg = json.dumps(resp) + "\n"
            writer.write(msg.encode("utf-8"))
            await writer.drain()

        writer.close()
        await writer.wait_closed()

    server = await asyncio.start_server(mock_handler, "127.0.0.1", 0)
    port = server.sockets[0].getsockname()[1]

    client = AsyncMeshClient()
    await client.connect_tcp("127.0.0.1", port)

    # 1. Heartbeat
    hb_resp = await client.heartbeat(worker_id="worker-test-1")
    assert hb_resp == "Acknowledged"

    # 2. Claim task
    task_id = str(uuid.uuid4())
    claim_resp = await client.claim_task(task_id, "A -> A")
    assert "TaskAssigned" in claim_resp
    assert claim_resp["TaskAssigned"]["task_id"] == task_id

    # 3. Submit proof
    strategy = {
        "root": {
            "current_move": {
                "id": 0,
                "player": "Proponent",
                "kind": "Question",
                "justifier": None,
                "payload": {"RootGoal": "A -> A"},
            },
            "children": [],
        }
    }
    proof_resp = await client.submit_proof(task_id, strategy)
    assert "ProofAccepted" in proof_resp

    await client.close()
    server.close()
    await server.wait_closed()

    assert len(received_commands) == 3
