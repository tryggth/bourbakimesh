"""Unit and integration tests for BourbakiMesh REST & WebSocket Telemetry API."""

from __future__ import annotations

import asyncio
import json
import pytest
from typing import Any, Dict, List, Optional

from bourbakimesh.api.server import create_app


async def asgi_http_request(
    app: Any,
    method: str = "GET",
    path: str = "/",
    body: Optional[Dict[str, Any]] = None,
) -> Dict[str, Any]:
    """Pure ASGI HTTP test helper without external dependencies."""
    body_bytes = json.dumps(body).encode("utf-8") if body is not None else b""
    headers = [
        (b"host", b"testserver"),
        (b"content-type", b"application/json"),
        (b"content-length", str(len(body_bytes)).encode("ascii")),
    ] if body is not None else [(b"host", b"testserver")]

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
async def test_api_status_endpoint() -> None:
    app = create_app(model_path="checkpoints/bourbaki_v2.pt")
    resp = await asgi_http_request(app, method="GET", path="/api/status")

    assert resp["status"] == 200
    data = resp["json"]
    assert data["status"] == "online"
    assert data["active_model"] == "checkpoints/bourbaki_v2.pt"
    assert data["peer_count"] >= 1
    assert data["total_blocks"] >= 1
    assert "hardware" in data
    assert data["hardware"]["cpu_cores"] >= 1


@pytest.mark.asyncio
async def test_api_ledger_endpoint() -> None:
    app = create_app()
    resp = await asgi_http_request(app, method="GET", path="/api/ledger")

    assert resp["status"] == 200
    data = resp["json"]
    assert "nodes" in data
    assert "edges" in data
    assert len(data["nodes"]) >= 3
    # Check genesis block
    genesis = data["nodes"][0]
    assert genesis["theorem_name"] == "Genesis"
    assert genesis["status"] == "certified"


@pytest.mark.asyncio
async def test_api_tournaments_endpoint() -> None:
    app = create_app()
    resp = await asgi_http_request(app, method="GET", path="/api/tournaments")

    assert resp["status"] == 200
    data = resp["json"]
    assert "rankings" in data
    assert len(data["rankings"]) >= 3
    names = [r["name"] for r in data["rankings"]]
    assert "bourbaki_v1.pt" in names
    assert "bourbaki_v2.pt" in names
    assert "bourbaki_v0.pt" in names


@pytest.mark.asyncio
async def test_api_prove_endpoint_conjunction() -> None:
    app = create_app()
    payload = {
        "theorem_name": "Mathlib.Logic.And.intro",
        "proposition": "A -> B -> A ∧ B",
    }
    resp = await asgi_http_request(app, method="POST", path="/api/prove", body=payload)

    assert resp["status"] == 200
    data = resp["json"]
    assert data["success"] is True
    assert data["theorem_name"] == "Mathlib.Logic.And.intro"
    assert len(data["dialogue"]) == 5
    assert "And.intro" in data["lean_code"]
    assert data["verified_in_lean"] is True
    assert data["time_ms"] > 0


@pytest.mark.asyncio
async def test_api_prove_endpoint_implication() -> None:
    app = create_app()
    payload = {
        "theorem_name": "Mathlib.Logic.Identity",
        "proposition": "P -> P",
    }
    resp = await asgi_http_request(app, method="POST", path="/api/prove", body=payload)

    assert resp["status"] == 200
    data = resp["json"]
    assert data["success"] is True
    assert data["theorem_name"] == "Mathlib.Logic.Identity"
    assert len(data["dialogue"]) == 3
    assert data["verified_in_lean"] is True


@pytest.mark.asyncio
async def test_websocket_telemetry_broadcasting() -> None:
    app = create_app()
    hub = app.state.telemetry_hub

    # Simulate WebSocket Client via ASGI
    received_messages: List[Dict[str, Any]] = []
    incoming_queue: asyncio.Queue[Dict[str, Any]] = asyncio.Queue()

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

    # Queue initial connect
    await incoming_queue.put({"type": "websocket.connect"})

    async def receive() -> Dict[str, Any]:
        return await incoming_queue.get()

    async def send(msg: Dict[str, Any]) -> None:
        if msg["type"] == "websocket.accept":
            pass
        elif msg["type"] == "websocket.send":
            text = msg.get("text")
            if text:
                received_messages.append(json.loads(text))

    ws_task = asyncio.create_task(app(scope, receive, send))

    # Give ASGI time to accept connection
    await asyncio.sleep(0.05)
    assert len(hub.active_connections) == 1

    # Trigger proof via HTTP endpoint
    payload = {
        "theorem_name": "Mathlib.Logic.ModusPonens",
        "proposition": "P -> (P -> Q) -> Q",
    }
    await asgi_http_request(app, method="POST", path="/api/prove", body=payload)

    # Let broadcast propagate
    await asyncio.sleep(0.05)

    # Clean disconnect
    await incoming_queue.put({"type": "websocket.disconnect", "code": 1000})
    await asyncio.sleep(0.05)
    ws_task.cancel()
    try:
        await ws_task
    except asyncio.CancelledError:
        pass

    # Verify telemetry events received
    event_types = [m["type"] for m in received_messages]
    assert "system_status" in event_types
    assert "mcts_step" in event_types
    assert "proof_attested" in event_types
