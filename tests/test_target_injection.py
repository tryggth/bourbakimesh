"""Integration tests for Top-Level Target Theorem Injection and Swarm Focus Protocol."""

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
async def test_get_current_target() -> None:
    app = create_app()
    resp = await asgi_http_request(app, method="GET", path="/api/target/current")
    assert resp["status"] == 200
    data = resp["json"]
    assert "name" in data
    assert "proposition" in data
    assert "status" in data
    assert data["priority"] >= 1


@pytest.mark.asyncio
async def test_set_target_custom_lean() -> None:
    app = create_app()
    payload = {
        "name": "Mathlib.Algebra.Group.mul_left_inv",
        "proposition": "a⁻¹ * a = 1",
        "lean_code": "theorem mul_left_inv (a : G) : a⁻¹ * a = 1 := by sorry",
        "priority": 150,
    }
    resp = await asgi_http_request(app, method="POST", path="/api/target/set", body=payload)
    assert resp["status"] == 200
    res_data = resp["json"]
    assert res_data["status"] == "target_updated"
    assert res_data["target"]["name"] == "Mathlib.Algebra.Group.mul_left_inv"
    assert res_data["target"]["proposition"] == "a⁻¹ * a = 1"
    assert res_data["target"]["priority"] == 150

    # Verify subsequent GET returns updated target
    cur_resp = await asgi_http_request(app, method="GET", path="/api/target/current")
    assert cur_resp["status"] == 200
    cur_data = cur_resp["json"]
    assert cur_data["name"] == "Mathlib.Algebra.Group.mul_left_inv"
    assert cur_data["proposition"] == "a⁻¹ * a = 1"


@pytest.mark.asyncio
async def test_set_target_auto_extract_signature() -> None:
    app = create_app()
    payload = {
        "name": "Custom.Commutativity",
        "lean_code": "theorem and_comm (a : A) (b : B) : A ∧ B -> B ∧ A := by sorry",
        "priority": 200,
    }
    resp = await asgi_http_request(app, method="POST", path="/api/target/set", body=payload)
    assert resp["status"] == 200
    target = resp["json"]["target"]
    assert "A ∧ B -> B ∧ A" in target["proposition"]
    assert target["priority"] == 200


@pytest.mark.asyncio
async def test_target_injection_websocket_broadcast() -> None:
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
    await asyncio.sleep(0.05)
    assert len(hub.active_connections) == 1

    # Inject target via HTTP POST
    payload = {
        "name": "Mathlib.Order.Basic.le_trans",
        "proposition": "a ≤ b -> b ≤ c -> a ≤ c",
        "priority": 120,
    }
    await asgi_http_request(app, method="POST", path="/api/target/set", body=payload)
    await asyncio.sleep(0.05)

    # Clean disconnect
    await incoming_queue.put({"type": "websocket.disconnect", "code": 1000})
    await asyncio.sleep(0.05)
    ws_task.cancel()
    try:
        await ws_task
    except asyncio.CancelledError:
        pass

    event_types = [m["type"] for m in received_messages]
    assert "system_status" in event_types
    assert "swarm_target_set" in event_types

    target_event = next(m for m in received_messages if m["type"] == "swarm_target_set")
    assert target_event["data"]["name"] == "Mathlib.Order.Basic.le_trans"
    assert target_event["data"]["proposition"] == "a ≤ b -> b ≤ c -> a ≤ c"
    assert target_event["data"]["priority"] == 120
