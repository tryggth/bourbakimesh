"""Asynchronous IPC client connecting Python Latent MCTS workers to Rust MeshCoordinator."""

from __future__ import annotations
import asyncio
import json
from typing import Any, Dict, Optional
import uuid
from bourbakimesh.models import BourbakiMuZero
from bourbakimesh.latent_mcts import LatentMCTS, MCTSConfig


class AsyncMeshClient:
    """Non-blocking IPC client connecting to the Rust coordinator over TCP or UDS."""

    def __init__(self) -> None:
        self.reader: Optional[asyncio.StreamReader] = None
        self.writer: Optional[asyncio.StreamWriter] = None

    async def connect_tcp(self, host: str = "127.0.0.1", port: int = 50051) -> None:
        """Establish TCP connection to the mesh coordinator."""
        self.reader, self.writer = await asyncio.open_connection(host, port)

    async def connect_uds(self, socket_path: str) -> None:
        """Establish Unix Domain Socket connection to the mesh coordinator."""
        self.reader, self.writer = await asyncio.open_unix_connection(socket_path)

    async def send_command(self, cmd: Dict[str, Any]) -> Dict[str, Any]:
        """Send a JSON-encoded command and await the newline-delimited JSON response."""
        if self.writer is None or self.reader is None:
            raise ConnectionError("AsyncMeshClient is not connected to a coordinator")

        message = json.dumps(cmd) + "\n"
        self.writer.write(message.encode("utf-8"))
        await self.writer.drain()

        line = await self.reader.readline()
        if not line:
            raise ConnectionResetError("Coordinator closed the connection")

        return json.loads(line.decode("utf-8").strip())

    async def heartbeat(self, worker_id: str = "python-worker-1") -> Dict[str, Any]:
        """Transmit heartbeat liveness ping."""
        return await self.send_command({"Heartbeat": {"worker_id": worker_id}})

    async def ping(self) -> Dict[str, Any]:
        """Send ping message."""
        return await self.send_command("Ping")

    async def claim_task(
        self,
        task_id: str,
        goal_statement: str,
        max_simulations: int = 100,
    ) -> Dict[str, Any]:
        """Claim or register a proving task with the coordinator."""
        cmd = {
            "ClaimTask": {
                "task_id": task_id,
                "goal_statement": goal_statement,
                "max_simulations": max_simulations,
            }
        }
        return await self.send_command(cmd)

    async def submit_proof(
        self,
        task_id: str,
        strategy: Dict[str, Any],
    ) -> Dict[str, Any]:
        """Submit an extracted winning strategy tree for verification and ledger insertion."""
        cmd = {
            "SubmitProof": {
                "task_id": task_id,
                "strategy": strategy,
            }
        }
        return await self.send_command(cmd)

    async def submit_refutation(
        self,
        task_id: str,
        counter_trace: Dict[str, Any],
    ) -> Dict[str, Any]:
        """Submit a refutation counter-trace."""
        cmd = {
            "SubmitRefutation": {
                "task_id": task_id,
                "counter_trace": counter_trace,
            }
        }
        return await self.send_command(cmd)

    async def close(self) -> None:
        """Close the active connection stream."""
        if self.writer is not None:
            self.writer.close()
            await self.writer.wait_closed()
            self.writer = None
            self.reader = None
