"""Live E2E Telemetry Feeder and Demo Runner for BourbakiMesh Web UI."""

from __future__ import annotations

import argparse
import hashlib
import json
import time
import urllib.error
import urllib.request
from typing import Any, Dict, List


DEMO_PROPOSITIONS = [
    {
        "name": "Mathlib.Logic.Basic.and_intro",
        "proposition": "A -> B -> A ∧ B",
        "tier": 1,
        "moves": [
            {
                "id": 0,
                "player": "P",
                "kind": "RootGoal",
                "justification_id": None,
                "payload": {"type": "RootGoal", "term_repr": "A ∧ B"},
                "p_view": [0],
                "o_view": [0],
            },
            {
                "id": 1,
                "player": "O",
                "kind": "Question",
                "justification_id": 0,
                "payload": {"type": "AttackConjunction", "branch": "Left"},
                "p_view": [0, 1],
                "o_view": [0, 1],
            },
            {
                "id": 2,
                "player": "P",
                "kind": "Assertion",
                "justification_id": 1,
                "payload": {"type": "ProvideWitness", "term_repr": "witness_a : A"},
                "p_view": [0, 1, 2],
                "o_view": [0, 1, 2],
            },
            {
                "id": 3,
                "player": "O",
                "kind": "Question",
                "justification_id": 0,
                "payload": {"type": "AttackConjunction", "branch": "Right"},
                "p_view": [0, 3],
                "o_view": [0, 1, 2, 3],
            },
            {
                "id": 4,
                "player": "P",
                "kind": "Assertion",
                "justification_id": 3,
                "payload": {"type": "ProvideWitness", "term_repr": "witness_b : B"},
                "p_view": [0, 3, 4],
                "o_view": [0, 1, 2, 3, 4],
            },
        ],
        "term": "fun (a : A) => fun (b : B) => And.intro a b",
    },
    {
        "name": "Mathlib.Logic.Basic.modus_ponens",
        "proposition": "P -> (P -> Q) -> Q",
        "tier": 1,
        "moves": [
            {
                "id": 0,
                "player": "P",
                "kind": "RootGoal",
                "justification_id": None,
                "payload": {"type": "RootGoal", "term_repr": "P -> (P -> Q) -> Q"},
                "p_view": [0],
                "o_view": [0],
            },
            {
                "id": 1,
                "player": "O",
                "kind": "Question",
                "justification_id": 0,
                "payload": {"type": "AttackHypothesis", "hyp_id": 0, "term_repr": "p : P"},
                "p_view": [0, 1],
                "o_view": [0, 1],
            },
            {
                "id": 2,
                "player": "O",
                "kind": "Question",
                "justification_id": 0,
                "payload": {"type": "AttackHypothesis", "hyp_id": 1, "term_repr": "f : P -> Q"},
                "p_view": [0, 2],
                "o_view": [0, 1, 2],
            },
            {
                "id": 3,
                "player": "P",
                "kind": "Assertion",
                "justification_id": 2,
                "payload": {"type": "Application", "func": "f", "arg": "p"},
                "p_view": [0, 2, 3],
                "o_view": [0, 1, 2, 3],
            },
        ],
        "term": "fun (p : P) => fun (f : P -> Q) => f p",
    },
    {
        "name": "Mathlib.Logic.Basic.trans_impl",
        "proposition": "(A -> B) -> (B -> C) -> A -> C",
        "tier": 2,
        "moves": [
            {
                "id": 0,
                "player": "P",
                "kind": "RootGoal",
                "justification_id": None,
                "payload": {"type": "RootGoal", "term_repr": "(A -> B) -> (B -> C) -> A -> C"},
                "p_view": [0],
                "o_view": [0],
            },
            {
                "id": 1,
                "player": "O",
                "kind": "Question",
                "justification_id": 0,
                "payload": {"type": "AttackHypothesis", "hyp_id": 0, "term_repr": "f : A -> B"},
                "p_view": [0, 1],
                "o_view": [0, 1],
            },
            {
                "id": 2,
                "player": "O",
                "kind": "Question",
                "justification_id": 0,
                "payload": {"type": "AttackHypothesis", "hyp_id": 1, "term_repr": "g : B -> C"},
                "p_view": [0, 2],
                "o_view": [0, 1, 2],
            },
            {
                "id": 3,
                "player": "O",
                "kind": "Question",
                "justification_id": 0,
                "payload": {"type": "AttackHypothesis", "hyp_id": 2, "term_repr": "a : A"},
                "p_view": [0, 3],
                "o_view": [0, 1, 2, 3],
            },
            {
                "id": 4,
                "player": "P",
                "kind": "Assertion",
                "justification_id": 2,
                "payload": {"type": "Application", "func": "g", "arg": "(f a)"},
                "p_view": [0, 2, 4],
                "o_view": [0, 1, 2, 3, 4],
            },
        ],
        "term": "fun (f : A -> B) => fun (g : B -> C) => fun (a : A) => g (f a)",
    },
    {
        "name": "Mathlib.Algebra.Group.mul_one",
        "proposition": "forall (a : G), a * 1 = a",
        "tier": 3,
        "moves": [
            {
                "id": 0,
                "player": "P",
                "kind": "RootGoal",
                "justification_id": None,
                "payload": {"type": "RootGoal", "term_repr": "forall (a : G), a * 1 = a"},
                "p_view": [0],
                "o_view": [0],
            },
            {
                "id": 1,
                "player": "O",
                "kind": "Question",
                "justification_id": 0,
                "payload": {"type": "OpenUniversal", "var": "a : G"},
                "p_view": [0, 1],
                "o_view": [0, 1],
            },
            {
                "id": 2,
                "player": "P",
                "kind": "Assertion",
                "justification_id": 1,
                "payload": {"type": "GroupAxiomDischarge", "axiom": "mul_one_axiom"},
                "p_view": [0, 1, 2],
                "o_view": [0, 1, 2],
            },
        ],
        "term": "fun (a : G) => Group.mul_one a",
    },
]


class DemoFeeder:
    """Streams live MCTS and proof events into the BourbakiMesh Telemetry API."""

    def __init__(self, host: str = "127.0.0.1", port: int = 8000) -> None:
        self.base_url = f"http://{host}:{port}"
        self.last_block_hash = "c5d6e7f8a9b0c1d2e3f4a5b6c7da3f58e99bc10123d4f5e6a7b8c9d0e1f2a3b4"

    def send_broadcast(self, event_type: str, data: Dict[str, Any]) -> bool:
        url = f"{self.base_url}/api/telemetry/broadcast"
        payload = json.dumps({"type": event_type, "data": data}).encode("utf-8")
        req = urllib.request.Request(
            url,
            data=payload,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=3) as resp:
                return resp.status == 200
        except Exception:
            return False

    def wait_for_server(self, max_retries: int = 30, delay: float = 1.0) -> bool:
        url = f"{self.base_url}/api/status"
        print(f"⏳ Waiting for BourbakiMesh server at {self.base_url}...")
        for i in range(max_retries):
            try:
                with urllib.request.urlopen(url, timeout=2) as resp:
                    if resp.status == 200:
                        print("✅ Connected to BourbakiMesh Telemetry API!")
                        return True
            except Exception:
                time.sleep(delay)
        print("❌ Server timed out.")
        return False

    def stream_theorem(self, theorem: Dict[str, Any], step_delay: float = 0.6) -> None:
        name = theorem["name"]
        prop = theorem["proposition"]
        tier = theorem["tier"]

        print(f"\n🔍 [MCTS Proof Search] Claiming Tier {tier} goal: {name} ({prop})")

        # 1. Search Dispatched
        self.send_broadcast(
            "mcts_step",
            {
                "phase": "root_evaluation",
                "theorem_name": name,
                "proposition": prop,
                "tier": tier,
                "simulations_target": 100,
                "model": "checkpoints/bourbaki_v2.pt",
            },
        )
        time.sleep(step_delay)

        # 2. Simulation rollouts
        for sim in [25, 50, 75, 100]:
            self.send_broadcast(
                "mcts_step",
                {
                    "phase": "rollout_search",
                    "simulations_completed": sim,
                    "throughput_sims_per_sec": 1042.8 + (sim % 15) * 12.3,
                    "top_action": "assertion_dispatched",
                    "policy_prior": 0.88 + (sim / 1000.0),
                    "value_estimate": 0.965,
                },
            )
            time.sleep(step_delay * 0.4)

        # 3. Game-Semantic Arena moves
        for move in theorem["moves"]:
            player_label = "Proponent (P)" if move["player"] == "P" else "Opponent (O)"
            color = "BLUE" if move["player"] == "P" else "ORANGE"
            print(f"   [{color}] Move #{move['id']} {player_label}: {move['kind']} -> {move['payload']['type']}")
            self.send_broadcast(
                "move_added",
                {
                    "theorem_name": name,
                    "move": move,
                    "p_view": move["p_view"],
                    "o_view": move["o_view"],
                },
            )
            time.sleep(step_delay * 0.7)

        # 4. Zero-Trust Lean 4 Verification & Block Attestation
        block_content = f"{name}:{prop}:{theorem['term']}:{time.time()}"
        block_hash = hashlib.sha256(block_content.encode("utf-8")).hexdigest()
        parents = [self.last_block_hash]
        self.last_block_hash = block_hash

        print(f"   🛡️ [Lean 4 Verification] Extracting strategy E(sigma) -> Soundness Certified 🟢")
        self.send_broadcast(
            "proof_attested",
            {
                "theorem_name": name,
                "proposition": prop,
                "extracted_term": theorem["term"],
                "lean_verified": True,
                "block_id": block_hash,
                "parents": parents,
                "byzantine_signatures": 5,
                "status": "certified",
            },
        )
        time.sleep(step_delay)


def main() -> None:
    parser = argparse.ArgumentParser(description="BourbakiMesh Live UI Demo Feeder")
    parser.add_argument("--host", type=str, default="127.0.0.1", help="API Host")
    parser.add_argument("--port", type=int, default=8000, help="API Port")
    parser.add_argument("--delay", type=float, default=0.6, help="Step delay in seconds")
    parser.add_argument("--loop", action="store_true", default=True, help="Continuously loop demo stream")
    parser.add_argument("--iterations", type=int, default=0, help="Max iterations (0 for infinite if loop is true)")

    args = parser.parse_args()

    feeder = DemoFeeder(host=args.host, port=args.port)
    if not feeder.wait_for_server():
        return

    iteration = 0
    print("\n======================================================================")
    print("🚀 BOURBAKIMESH LIVE DEMO FEEDER ACTIVE")
    print("   Streaming game-semantic dialogue moves and MCTS telemetry...")
    print("======================================================================")

    try:
        while True:
            iteration += 1
            print(f"\n--- Demo Cycle #{iteration} ---")
            for thm in DEMO_PROPOSITIONS:
                feeder.stream_theorem(thm, step_delay=args.delay)
                time.sleep(args.delay * 2)

            if not args.loop or (args.iterations > 0 and iteration >= args.iterations):
                break
    except KeyboardInterrupt:
        print("\nDemo feeder stopped.")


if __name__ == "__main__":
    main()
