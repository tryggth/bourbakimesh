"""Game-semantic Lorenzen / Hyland-Ong dialogue arena state."""

from __future__ import annotations
from enum import Enum
from typing import List, Optional
import networkx as nx
from pydantic import BaseModel, Field


class DialoguePolarity(str, Enum):
    """Polarity of the dialogue game participant."""

    PROPONENT = "P"
    OPPONENT = "O"


class MoveKind(str, Enum):
    """Dialogue move classification."""

    QUESTION = "Q"
    ASSERTION = "A"


class DialogueMove(BaseModel):
    """A move in the dialogue game arena."""

    move_id: int
    polarity: DialoguePolarity
    kind: MoveKind
    label: str
    target_move_id: Optional[int] = None


class ArenaState(BaseModel):
    """Represents a state in the Lorenzen / Hyland-Ong dialogue arena."""

    moves: List[DialogueMove] = Field(default_factory=list)
    current_polarity: DialoguePolarity = DialoguePolarity.OPPONENT

    def add_move(self, move: DialogueMove) -> None:
        """Append a move and alternate polarity according to game rules."""
        if move.polarity != self.current_polarity:
            raise ValueError(
                f"Invalid move polarity: expected {self.current_polarity}, got {move.polarity}"
            )
        self.moves.append(move)
        # Alternate polarity
        self.current_polarity = (
            DialoguePolarity.OPPONENT
            if self.current_polarity == DialoguePolarity.PROPONENT
            else DialoguePolarity.PROPONENT
        )

    def to_graph(self) -> nx.DiGraph:
        """Convert dialogue sequence into a game-semantic justification graph."""
        g = nx.DiGraph()
        for m in self.moves:
            g.add_node(
                m.move_id,
                polarity=m.polarity.value,
                kind=m.kind.value,
                label=m.label,
            )
            if m.target_move_id is not None:
                g.add_edge(m.move_id, m.target_move_id)
        return g
