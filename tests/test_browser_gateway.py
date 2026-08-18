"""Integration tests for Volunteer In-Browser Gateway Bridge (Issue #30, #31, #32)."""

from __future__ import annotations

import asyncio
import json
from typing import Any, Dict, List, Optional
import pytest

from bourbakimesh.api.server import create_app


async def asgi_http_request(
    app: Any,
    method: str = "GET",
    path: str = "/",
    body: Optional[Dict[str, Any]] = None,
) -> Dict[str, Any]:
    """Pure ASGI HTTP test helper."""
    body_bytes = json.dumps(body).encode("utf-8") if body is not None else b""
    headers = (
        [
            (b"host", b"testserver"),
            (b"content-type", b"application/json"),
            (b"content-length", str(len(body_bytes)).encode("ascii")),
        ]
        if body is not None
        else [(b"host", b"testserver")]
    )

    scope = {
        "type": "http",
        "asgi": {"version": "3.0", "spec_version": "2.1"},
        "http_version": "1.1",
        "method": method,
        "path": path,
        "raw_path": path.encode("ascii"),
        "query_string": b"",
        "headers": headers,
        "server": ("127.0.0.1", 8000),
        "client": ("127.0.0.1", 54321),
        "scheme": "http",
    }

    sent_body = False

    async def receive() -> Dict[str, Any]:
        nonlocal sent_body
        if not sent_body:
            sent_body = True
            return {"type": "http.request", "body": body_bytes, "more_body": False}
        return {"type": "http.disconnect"}

    res = {"status": 0, "headers": {}, "body": b""}

    async def send(msg: Dict[str, Any]) -> None:
        if msg["type"] == "http.response.start":
            res["status"] = msg["status"]
            res["headers"] = dict(msg.get("headers", []))
        elif msg["type"] == "http.response.body":
            res["body"] += msg.get("body", b"")

    await app(scope, receive, send)
    try:
        res_json = json.loads(res["body"].decode("utf-8")) if res["body"] else None
    except Exception:
        res_json = None

    return {"status": res["status"], "headers": res["headers"], "json": res_json, "body": res["body"]}


@pytest.mark.asyncio
async def test_rest_tasks_open_and_claim() -> None:
    app = create_app()

    # 1. Fetch initial open tasks
    open_resp = await asgi_http_request(app, method="GET", path="/api/tasks/open")
    assert open_resp["status"] == 200
    assert open_resp["json"]["count"] >= 1
    initial_task = open_resp["json"]["tasks"][0]
    task_id = initial_task["task_id"]

    # 2. Register worker
    reg_resp = await asgi_http_request(
        app,
        method="POST",
        path="/api/workers/register",
        body={"peer_id": "browser-peer-test-1", "capabilities": {"provider": "webgpu", "sims_per_sec": 650.0}},
    )
    assert reg_resp["status"] == 200
    assert reg_resp["json"]["status"] == "registered"

    # 3. Claim open task
    claim_resp = await asgi_http_request(
        app,
        method="POST",
        path="/api/tasks/claim",
        body={"peer_id": "browser-peer-test-1"},
    )
    assert claim_resp["status"] == 200
    claimed_task = claim_resp["json"]["task"]
    assert claimed_task["task_id"] == task_id
    assert claimed_task["status"] == "claimed"
    assert claimed_task["claimed_by"] == "browser-peer-test-1"

    # 4. Submit Proof
    proof_payload = {
        "task_id": task_id,
        "theorem_name": claimed_task["theorem_name"],
        "proposition": claimed_task["proposition"],
        "extracted_term": "fun (p : P) => p",
        "signature": "3045022100abcd1234ef5678",
        "public_key": "04aabbccddeeff",
        "prover_peer_id": "browser-peer-test-1",
    }
    submit_resp = await asgi_http_request(app, method="POST", path="/api/proofs/submit", body=proof_payload)
    assert submit_resp["status"] == 200
    submit_json = submit_resp["json"]
    assert submit_json["status"] == "proof_accepted"
    assert submit_json["lean_verified"] is True
    assert len(submit_json["block_id"]) == 64

    # 5. Verify DAG ledger contains new block
    ledger_resp = await asgi_http_request(app, method="GET", path="/api/ledger")
    assert ledger_resp["status"] == 200
    block_ids = [n["id"] for n in ledger_resp["json"]["nodes"]]
    assert submit_json["block_id"] in block_ids


@pytest.mark.asyncio
async def test_websocket_gateway_full_cycle() -> None:
    app = create_app()
    hub = app.state.telemetry_hub

    received_messages: List[Dict[str, Any]] = []
    incoming_queue: asyncio.Queue = asyncio.Queue()

    scope = {
        "type": "websocket",
        "asgi": {"version": "3.0"},
        "http_version": "1.1",
        "path": "/ws/telemetry",
        "raw_path": b"/ws/telemetry",
        "query_string": b"",
        "headers": [(b"host", b"testserver")],
        "server": ("127.0.0.1", 8000),
        "client": ("127.0.0.1", 54321),
        "scheme": "ws",
        "subprotocols": [],
    }

    await incoming_queue.put({"type": "websocket.connect"})

    async def receive() -> Dict[str, Any]:
        return await incoming_queue.get()

    async def send(msg: Dict[str, Any]) -> None:
        if msg["type"] == "websocket.send":
            text = msg.get("text")
            if text:
                received_messages.append(json.loads(text))

    ws_task = asyncio.create_task(app(scope, receive, send))
    await asyncio.sleep(0.05)
    assert len(hub.active_connections) == 1

    peer_id = "browser-12D3KooWtestWebsocket"

    # Step 1: Register browser worker over WebSocket
    reg_msg = {
        "type": "register_worker",
        "peer_id": peer_id,
        "capabilities": {"provider": "webgpu", "sims_per_sec": 820.5, "power_mode": "max"},
    }
    await incoming_queue.put({"type": "websocket.receive", "text": json.dumps(reg_msg)})
    await asyncio.sleep(0.05)

    assert any(m.get("type") == "worker_registered" and m.get("peer_id") == peer_id for m in received_messages)

    # Step 2: Claim task over WebSocket
    claim_msg = {
        "type": "claim_task",
        "peer_id": peer_id,
    }
    await incoming_queue.put({"type": "websocket.receive", "text": json.dumps(claim_msg)})
    await asyncio.sleep(0.05)

    task_event = next(m for m in received_messages if m.get("type") == "task_assigned")
    claimed_task = task_event["task"]
    assert claimed_task["claimed_by"] == peer_id

    # Step 3: Submit proof over WebSocket
    submit_msg = {
        "type": "submit_proof",
        "task_id": claimed_task["task_id"],
        "theorem_name": claimed_task["theorem_name"],
        "proposition": claimed_task["proposition"],
        "extracted_term": "fun (p : P) => p",
        "signature": "30450220abcdef",
        "public_key": "04123456",
        "prover_peer_id": peer_id,
    }
    await incoming_queue.put({"type": "websocket.receive", "text": json.dumps(submit_msg)})
    await asyncio.sleep(0.05)

    accept_event = next(m for m in received_messages if m.get("type") == "proof_accepted")
    assert accept_event["task_id"] == claimed_task["task_id"]
    assert accept_event["lean_verified"] is True

    # Step 4: Verify proof_attested was broadcasted
    attested_broadcast = next(m for m in received_messages if m.get("type") == "proof_attested")
    assert attested_broadcast["data"]["prover"] == peer_id
    assert attested_broadcast["data"]["id"] == accept_event["block_id"]

    # Clean disconnect
    await incoming_queue.put({"type": "websocket.disconnect", "code": 1000})
    await asyncio.sleep(0.05)
    ws_task.cancel()
    try:
        await ws_task
    except asyncio.CancelledError:
        pass
