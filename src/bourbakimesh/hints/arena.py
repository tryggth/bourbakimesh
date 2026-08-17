"""Arena Cut and Auxiliary Lemma Injection Helpers."""

from __future__ import annotations
from typing import Any, Dict, List, Optional
from pydantic import BaseModel, Field


class CutSpec(BaseModel):
    """Specification of an auxiliary lemma cut to inject into dialogue game arenas."""

    lemma_id: int = Field(default=0, ge=0)
    statement: str = Field(..., description="Proposition or type of the lemma, e.g. 'A -> B'")
    premise_requirements: List[str] = Field(default_factory=list)
    tactic_hint: Optional[str] = Field(default=None)


class ArenaCutInjector:
    """Helper for constructing and validating game-semantic cut lemma structures."""

    @staticmethod
    def build_cut_payload(lemma_id: int, statement: str) -> Dict[str, Any]:
        """Construct a JSON-serializable Move payload for an AssertCutLemma move."""
        return {
            "AssertCutLemma": {
                "lemma_id": lemma_id,
                "statement": statement.strip(),
            }
        }

    @staticmethod
    def build_let_binding_lean(
        lemma_id: int,
        lemma_type: str,
        lemma_proof: str,
        continuation_body: str,
    ) -> str:
        """Construct a verified Lean 4 let-binding expression."""
        return f"let lem_{lemma_id} : {lemma_type.strip()} := {lemma_proof.strip()}; {continuation_body.strip()}"

    @staticmethod
    def inject_cut_into_trace(
        existing_moves: List[Dict[str, Any]],
        cut_spec: CutSpec,
    ) -> List[Dict[str, Any]]:
        """Prepend or wrap existing moves under an AssertCutLemma move."""
        cut_move = {
            "id": 0,
            "player": "Proponent",
            "kind": "Answer",
            "justifier": None,
            "payload": ArenaCutInjector.build_cut_payload(
                cut_spec.lemma_id, cut_spec.statement
            ),
        }
        # Shift subsequent move ids by 1
        shifted_moves = []
        for m in existing_moves:
            m_copy = dict(m)
            m_copy["id"] = m_copy.get("id", 0) + 1
            if m_copy.get("justifier") is not None:
                m_copy["justifier"] = m_copy["justifier"] + 1
            shifted_moves.append(m_copy)

        return [cut_move] + shifted_moves
