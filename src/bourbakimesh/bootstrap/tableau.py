"""Analytic First-Order Semantic Tableau Solver for automated theorem proving."""

from __future__ import annotations
from dataclasses import dataclass, field
from typing import List, Optional, Set, Tuple


@dataclass(frozen=True)
class Formula:
    """Base class for first-order logic formulas."""
    pass


@dataclass(frozen=True)
class Atom(Formula):
    name: str

    def __str__(self) -> str:
        return self.name


@dataclass(frozen=True)
class Not(Formula):
    inner: Formula

    def __str__(self) -> str:
        return f"¬({self.inner})"


@dataclass(frozen=True)
class And(Formula):
    left: Formula
    right: Formula

    def __str__(self) -> str:
        return f"({self.left} ∧ {self.right})"


@dataclass(frozen=True)
class Or(Formula):
    left: Formula
    right: Formula

    def __str__(self) -> str:
        return f"({self.left} ∨ {self.right})"


@dataclass(frozen=True)
class Implies(Formula):
    antecedent: Formula
    consequent: Formula

    def __str__(self) -> str:
        return f"({self.antecedent} → {self.consequent})"


@dataclass(frozen=True)
class Forall(Formula):
    var: str
    body: Formula

    def __str__(self) -> str:
        return f"∀{self.var}. ({self.body})"


@dataclass(frozen=True)
class Exists(Formula):
    var: str
    body: Formula

    def __str__(self) -> str:
        return f"∃{self.var}. ({self.body})"


@dataclass(frozen=True)
class SignedFormula:
    """A signed formula T(A) or F(A) in Smullyan-style semantic tableaux."""
    formula: Formula
    sign: bool  # True for T, False for F

    def __str__(self) -> str:
        prefix = "T" if self.sign else "F"
        return f"{prefix}({self.formula})"


@dataclass
class TableauNode:
    """A node in a semantic tableau proof tree."""
    signed: SignedFormula
    children: List[TableauNode] = field(default_factory=list)
    closed: bool = False
    closure_atom: Optional[str] = None


class TableauSolver:
    """Analytic semantic tableau refutation prover."""

    def __init__(self, max_depth: int = 50) -> None:
        self.max_depth = max_depth

    def prove(self, formula: Formula) -> Optional[TableauNode]:
        """Attempt to prove a formula by refuting F(formula)."""
        root_signed = SignedFormula(formula, sign=False)
        root_node = TableauNode(signed=root_signed)

        success = self._expand_branch([root_node], {root_signed}, 0)
        if success and root_node.closed:
            return root_node
        return None

    def _expand_branch(
        self,
        current_path: List[TableauNode],
        formulas_on_path: Set[SignedFormula],
        depth: int,
    ) -> bool:
        if depth > self.max_depth:
            return False

        leaf = current_path[-1]

        # 1. Check for immediate contradiction on this branch
        for sf in formulas_on_path:
            if isinstance(sf.formula, Atom):
                complement = SignedFormula(sf.formula, not sf.sign)
                if complement in formulas_on_path:
                    leaf.closed = True
                    leaf.closure_atom = sf.formula.name
                    return True

        # 2. Find an unused non-atomic formula to expand
        for sf in list(formulas_on_path):
            f = sf.formula
            sign = sf.sign

            # Alpha rules (Deterministic expansion)
            if isinstance(f, Not):
                new_sf = SignedFormula(f.inner, not sign)
                if new_sf not in formulas_on_path:
                    child_node = TableauNode(signed=new_sf)
                    leaf.children.append(child_node)
                    if self._expand_branch(
                        current_path + [child_node],
                        formulas_on_path | {new_sf},
                        depth + 1,
                    ):
                        leaf.closed = True
                        return True
                    leaf.children.pop()

            elif isinstance(f, And) and sign:  # T(A ∧ B) -> T(A), T(B)
                sf_a = SignedFormula(f.left, True)
                sf_b = SignedFormula(f.right, True)
                if sf_a not in formulas_on_path or sf_b not in formulas_on_path:
                    c1 = TableauNode(signed=sf_a)
                    c2 = TableauNode(signed=sf_b)
                    c1.children.append(c2)
                    leaf.children.append(c1)
                    if self._expand_branch(
                        current_path + [c1, c2],
                        formulas_on_path | {sf_a, sf_b},
                        depth + 1,
                    ):
                        leaf.closed = True
                        return True
                    leaf.children.pop()

            elif isinstance(f, Or) and not sign:  # F(A ∨ B) -> F(A), F(B)
                sf_a = SignedFormula(f.left, False)
                sf_b = SignedFormula(f.right, False)
                if sf_a not in formulas_on_path or sf_b not in formulas_on_path:
                    c1 = TableauNode(signed=sf_a)
                    c2 = TableauNode(signed=sf_b)
                    c1.children.append(c2)
                    leaf.children.append(c1)
                    if self._expand_branch(
                        current_path + [c1, c2],
                        formulas_on_path | {sf_a, sf_b},
                        depth + 1,
                    ):
                        leaf.closed = True
                        return True
                    leaf.children.pop()

            elif isinstance(f, Implies) and not sign:  # F(A → B) -> T(A), F(B)
                sf_a = SignedFormula(f.antecedent, True)
                sf_b = SignedFormula(f.consequent, False)
                if sf_a not in formulas_on_path or sf_b not in formulas_on_path:
                    c1 = TableauNode(signed=sf_a)
                    c2 = TableauNode(signed=sf_b)
                    c1.children.append(c2)
                    leaf.children.append(c1)
                    if self._expand_branch(
                        current_path + [c1, c2],
                        formulas_on_path | {sf_a, sf_b},
                        depth + 1,
                    ):
                        leaf.closed = True
                        return True
                    leaf.children.pop()

            # Beta rules (Branching splits)
            elif isinstance(f, Implies) and sign:  # T(A → B) -> F(A) | T(B)
                sf_a = SignedFormula(f.antecedent, False)
                sf_b = SignedFormula(f.consequent, True)
                if sf_a not in formulas_on_path and sf_b not in formulas_on_path:
                    branch_a = TableauNode(signed=sf_a)
                    branch_b = TableauNode(signed=sf_b)

                    closed_a = self._expand_branch(
                        current_path + [branch_a],
                        formulas_on_path | {sf_a},
                        depth + 1,
                    )
                    closed_b = self._expand_branch(
                        current_path + [branch_b],
                        formulas_on_path | {sf_b},
                        depth + 1,
                    )

                    if closed_a and closed_b:
                        leaf.children = [branch_a, branch_b]
                        leaf.closed = True
                        return True

            elif isinstance(f, Or) and sign:  # T(A ∨ B) -> T(A) | T(B)
                sf_a = SignedFormula(f.left, True)
                sf_b = SignedFormula(f.right, True)
                if sf_a not in formulas_on_path and sf_b not in formulas_on_path:
                    branch_a = TableauNode(signed=sf_a)
                    branch_b = TableauNode(signed=sf_b)

                    closed_a = self._expand_branch(
                        current_path + [branch_a],
                        formulas_on_path | {sf_a},
                        depth + 1,
                    )
                    closed_b = self._expand_branch(
                        current_path + [branch_b],
                        formulas_on_path | {sf_b},
                        depth + 1,
                    )

                    if closed_a and closed_b:
                        leaf.children = [branch_a, branch_b]
                        leaf.closed = True
                        return True

            elif isinstance(f, And) and not sign:  # F(A ∧ B) -> F(A) | F(B)
                sf_a = SignedFormula(f.left, False)
                sf_b = SignedFormula(f.right, False)
                if sf_a not in formulas_on_path and sf_b not in formulas_on_path:
                    branch_a = TableauNode(signed=sf_a)
                    branch_b = TableauNode(signed=sf_b)

                    closed_a = self._expand_branch(
                        current_path + [branch_a],
                        formulas_on_path | {sf_a},
                        depth + 1,
                    )
                    closed_b = self._expand_branch(
                        current_path + [branch_b],
                        formulas_on_path | {sf_b},
                        depth + 1,
                    )

                    if closed_a and closed_b:
                        leaf.children = [branch_a, branch_b]
                        leaf.closed = True
                        return True

        return False
