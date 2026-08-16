"""Transpiler converting Closed Tableau trees into Bourbaki IR Dialogue StrategyTrees and PlayTraces."""

from __future__ import annotations
from typing import Any, Dict, List, Optional
from bourbakimesh.bootstrap.tableau import Atom, Formula, Implies, And, TableauNode


class TableauToDialogueTranspiler:
    """Converts analytic semantic tableau proofs into Lorenzen/Hyland-Ong dialogue arena strategies."""

    def transpile_to_strategy(self, formula: Formula, tableau: TableauNode) -> Dict[str, Any]:
        """Convert a closed tableau into a structured StrategyTree JSON representation."""
        goal_str = str(formula)
        root_move = {
            "id": 0,
            "player": "Proponent",
            "kind": "Question",
            "justifier": None,
            "payload": {"RootGoal": goal_str},
        }

        root_node = {
            "current_move": root_move,
            "children": [],
        }

        # Recursively construct dialogue interaction tree
        self._build_strategy_node(formula, tableau, root_node, current_step=1, parent_step=0)

        return {"root": root_node}

    def transpile_to_play_trace(self, formula: Formula, tableau: TableauNode) -> Dict[str, Any]:
        """Flatten the principal branch of a closed tableau into a valid alternating PlayTrace."""
        strategy = self.transpile_to_strategy(formula, tableau)
        moves: List[Dict[str, Any]] = []

        def collect_linear_path(node: Dict[str, Any]) -> None:
            if node and "current_move" in node:
                moves.append(node["current_move"])
                if node.get("children"):
                    collect_linear_path(node["children"][0])

        if strategy.get("root"):
            collect_linear_path(strategy["root"])

        return {"moves": moves}

    def _build_strategy_node(
        self,
        formula: Formula,
        tableau_node: TableauNode,
        parent_dict: Dict[str, Any],
        current_step: int,
        parent_step: int,
    ) -> int:
        if isinstance(formula, Implies):
            # Implication intro: Opponent challenges antecedent (hypothesis)
            hyp_name = str(formula.antecedent)
            opp_move = {
                "id": current_step,
                "player": "Opponent",
                "kind": "Question",
                "justifier": parent_step,
                "payload": {"InstantiateUniversal": {"term_repr": f"h_{current_step} : {hyp_name}"}},
            }
            opp_node = {
                "current_move": opp_move,
                "children": [],
            }
            parent_dict["children"].append(opp_node)

            # Continue into consequent
            next_step = current_step + 1
            if isinstance(formula.consequent, Implies):
                return self._build_strategy_node(
                    formula.consequent,
                    tableau_node,
                    opp_node,
                    current_step=next_step,
                    parent_step=current_step,
                )
            else:
                # Leaf answer discharging the goal
                ans_move = {
                    "id": next_step,
                    "player": "Proponent",
                    "kind": "Answer",
                    "justifier": current_step,
                    "payload": {"ProvideWitness": {"term_repr": f"h_{current_step}"}},
                }
                opp_node["children"].append({"current_move": ans_move, "children": []})
                return next_step + 1

        elif isinstance(formula, And):
            # Conjunction introduction: branch into Left and Right
            left_move = {
                "id": current_step,
                "player": "Opponent",
                "kind": "Question",
                "justifier": parent_step,
                "payload": {"AttackConjunction": {"branch": "Left"}},
            }
            left_child = {
                "current_move": left_move,
                "children": [
                    {
                        "current_move": {
                            "id": current_step + 1,
                            "player": "Proponent",
                            "kind": "Answer",
                            "justifier": current_step,
                            "payload": {"ProvideWitness": {"term_repr": f"witness_{formula.left}"}},
                        },
                        "children": [],
                    }
                ],
            }

            right_move = {
                "id": current_step + 2,
                "player": "Opponent",
                "kind": "Question",
                "justifier": parent_step,
                "payload": {"AttackConjunction": {"branch": "Right"}},
            }
            right_child = {
                "current_move": right_move,
                "children": [
                    {
                        "current_move": {
                            "id": current_step + 3,
                            "player": "Proponent",
                            "kind": "Answer",
                            "justifier": current_step + 2,
                            "payload": {"ProvideWitness": {"term_repr": f"witness_{formula.right}"}},
                        },
                        "children": [],
                    }
                ],
            }

            parent_dict["children"].extend([left_child, right_child])
            return current_step + 4

        else:
            # Atomic formula discharge
            ans_move = {
                "id": current_step,
                "player": "Proponent",
                "kind": "Answer",
                "justifier": parent_step,
                "payload": {"ProvideWitness": {"term_repr": f"witness_{formula}"}},
            }
            parent_dict["children"].append({"current_move": ans_move, "children": []})
            return current_step + 1
