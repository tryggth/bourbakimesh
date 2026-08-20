/**
 * BourbakiMesh Generalized Constructive Proposition CIC Solver.
 *
 * Implements a sound, recursive type-directed proof synthesis engine for
 * constructive intuitionistic propositional and dependent logic formulas in the
 * Calculus of Inductive Constructions (CIC) AST.
 */

import { CicExpr, CicLevel } from '../config/models';

export interface CicHypothesis {
  name: string;
  type: CicExpr;
  isPropVal?: boolean;
}

export interface CicSolverResult {
  proofTerm: CicExpr;
  reasoning: string;
}

/**
 * Checks if a CicExpr represents the Prop universe sort (Sort 0).
 */
export function isPropSort(e: CicExpr): boolean {
  if (typeof e !== 'object' || e === null) return false;
  return 'Sort' in e && e.Sort === 'Zero';
}

/**
 * Checks if a CicExpr is False.
 */
export function isFalse(e: CicExpr): boolean {
  if (typeof e !== 'object' || e === null) return false;
  if ('Const' in e && e.Const[0] === 'False') return true;
  if ('FVar' in e && e.FVar === 'False') return true;
  return false;
}

/**
 * Matches a conjunction formula: App(App(Const("And", []), left), right).
 */
export function matchAnd(e: CicExpr): { left: CicExpr; right: CicExpr } | null {
  if (typeof e !== 'object' || e === null || !('App' in e)) return null;
  const [f1, right] = e.App;
  if (typeof f1 !== 'object' || f1 === null || !('App' in f1)) return null;
  const [f0, left] = f1.App;
  if (typeof f0 === 'object' && f0 !== null && 'Const' in f0 && f0.Const[0] === 'And') {
    return { left, right };
  }
  return null;
}

/**
 * Matches a disjunction formula: App(App(Const("Or", []), left), right).
 */
export function matchOr(e: CicExpr): { left: CicExpr; right: CicExpr } | null {
  if (typeof e !== 'object' || e === null || !('App' in e)) return null;
  const [f1, right] = e.App;
  if (typeof f1 !== 'object' || f1 === null || !('App' in f1)) return null;
  const [f0, left] = f1.App;
  if (typeof f0 === 'object' && f0 !== null && 'Const' in f0 && f0.Const[0] === 'Or') {
    return { left, right };
  }
  return null;
}

/**
 * Matches an equality formula: App(App(App(Const("Eq", levels), type), lhs), rhs).
 */
export function matchEq(e: CicExpr): { type: CicExpr; lhs: CicExpr; rhs: CicExpr; levels: CicLevel[] } | null {
  if (typeof e !== 'object' || e === null || !('App' in e)) return null;
  const [f2, rhs] = e.App;
  if (typeof f2 !== 'object' || f2 === null || !('App' in f2)) return null;
  const [f1, lhs] = f2.App;
  if (typeof f1 !== 'object' || f1 === null || !('App' in f1)) return null;
  const [f0, type] = f1.App;
  if (typeof f0 === 'object' && f0 !== null && 'Const' in f0 && f0.Const[0] === 'Eq') {
    return { type, lhs, rhs, levels: f0.Const[1] || [] };
  }
  return null;
}

/**
 * Shifts loose De Bruijn indices in an expression by `amount` above `cutoff`.
 */
export function liftCicExpr(expr: CicExpr, amount: number, cutoff = 0): CicExpr {
  if (amount === 0 || typeof expr !== 'object' || expr === null) return expr;

  if ('BVar' in expr) {
    return expr.BVar >= cutoff ? { BVar: expr.BVar + amount } : { BVar: expr.BVar };
  }
  if ('App' in expr) {
    return { App: [liftCicExpr(expr.App[0], amount, cutoff), liftCicExpr(expr.App[1], amount, cutoff)] };
  }
  if ('Lam' in expr) {
    return {
      Lam: [
        expr.Lam[0],
        liftCicExpr(expr.Lam[1], amount, cutoff),
        liftCicExpr(expr.Lam[2], amount, cutoff + 1),
      ],
    };
  }
  if ('ForallE' in expr) {
    return {
      ForallE: [
        expr.ForallE[0],
        liftCicExpr(expr.ForallE[1], amount, cutoff),
        liftCicExpr(expr.ForallE[2], amount, cutoff + 1),
      ],
    };
  }
  if ('LetE' in expr) {
    return {
      LetE: [
        expr.LetE[0],
        liftCicExpr(expr.LetE[1], amount, cutoff),
        liftCicExpr(expr.LetE[2], amount, cutoff),
        liftCicExpr(expr.LetE[3], amount, cutoff + 1),
      ],
    };
  }
  return expr;
}

/**
 * Checks structural equality of two CIC expressions.
 */
export function exprEquals(a: CicExpr, b: CicExpr): boolean {
  if (a === b) return true;
  if (typeof a !== 'object' || a === null || typeof b !== 'object' || b === null) return false;

  if ('BVar' in a && 'BVar' in b) return a.BVar === b.BVar;
  if ('FVar' in a && 'FVar' in b) return a.FVar === b.FVar;
  if ('Sort' in a && 'Sort' in b) return JSON.stringify(a.Sort) === JSON.stringify(b.Sort);
  if ('Const' in a && 'Const' in b) {
    return a.Const[0] === b.Const[0];
  }
  if ('App' in a && 'App' in b) {
    return exprEquals(a.App[0], b.App[0]) && exprEquals(a.App[1], b.App[1]);
  }
  if ('Lam' in a && 'Lam' in b) {
    return exprEquals(a.Lam[1], b.Lam[1]) && exprEquals(a.Lam[2], b.Lam[2]);
  }
  if ('ForallE' in a && 'ForallE' in b) {
    return exprEquals(a.ForallE[1], b.ForallE[1]) && exprEquals(a.ForallE[2], b.ForallE[2]);
  }
  return JSON.stringify(a) === JSON.stringify(b);
}

/**
 * Substitutes `replacement` for bound variable `BVar(targetIdx)` in `expr`,
 * adjusting De Bruijn indices of free variables above `targetIdx`.
 */
export function substCicExpr(expr: CicExpr, replacement: CicExpr, targetIdx = 0): CicExpr {
  if (typeof expr !== 'object' || expr === null) return expr;

  if ('BVar' in expr) {
    if (expr.BVar === targetIdx) {
      return liftCicExpr(replacement, targetIdx, 0);
    } else if (expr.BVar > targetIdx) {
      return { BVar: expr.BVar - 1 };
    } else {
      return { BVar: expr.BVar };
    }
  }

  if ('App' in expr) {
    return {
      App: [
        substCicExpr(expr.App[0], replacement, targetIdx),
        substCicExpr(expr.App[1], replacement, targetIdx),
      ],
    };
  }

  if ('Lam' in expr) {
    return {
      Lam: [
        expr.Lam[0],
        substCicExpr(expr.Lam[1], replacement, targetIdx),
        substCicExpr(expr.Lam[2], replacement, targetIdx + 1),
      ],
    };
  }

  if ('ForallE' in expr) {
    return {
      ForallE: [
        expr.ForallE[0],
        substCicExpr(expr.ForallE[1], replacement, targetIdx),
        substCicExpr(expr.ForallE[2], replacement, targetIdx + 1),
      ],
    };
  }

  if ('LetE' in expr) {
    return {
      LetE: [
        expr.LetE[0],
        substCicExpr(expr.LetE[1], replacement, targetIdx),
        substCicExpr(expr.LetE[2], replacement, targetIdx),
        substCicExpr(expr.LetE[3], replacement, targetIdx + 1),
      ],
    };
  }

  return expr;
}

/**
 * Helper to construct an application spine: f a1 a2 ... an
 */
export function mkApp(fun: CicExpr, args: CicExpr[]): CicExpr {
  return args.reduce((acc, arg) => ({ App: [acc, arg] }), fun);
}

/**
 * Main General-Purpose Recursive Constructive Proposition CIC Solver.
 *
 * Given a target goal type (e.g. `ForallE(...)`), automatically synthesizes
 * a closed, well-typed CIC lambda term.
 */
export function solveConstructiveCic(targetType: CicExpr): CicSolverResult | null {
  const result = solveIntroduction(targetType, [], 0);
  if (!result) return null;
  return {
    proofTerm: result.proof,
    reasoning: result.reasoning,
  };
}

/**
 * Recursive Introduction Pass: Peels outer `ForallE` binders into matching `Lam` abstractions
 * while maintaining the De Bruijn index context.
 */
function solveIntroduction(
  type: CicExpr,
  ctx: CicHypothesis[],
  depth: number
): { proof: CicExpr; reasoning: string } | null {
  if (type && typeof type === 'object' && 'ForallE' in type) {
    const [binderName, domainType, codomainType] = type.ForallE;

    // Extend context: new binder is at BVar(0), all existing hypotheses shifted by 1
    const newCtx: CicHypothesis[] = [
      {
        name: binderName,
        type: liftCicExpr(domainType, 1, 0),
        isPropVal: !isPropSort(domainType),
      },
      ...ctx.map((h) => ({
        name: h.name,
        type: liftCicExpr(h.type, 1, 0),
        isPropVal: h.isPropVal,
      })),
    ];

    const innerResult = solveIntroduction(codomainType, newCtx, depth + 1);
    if (!innerResult) return null;

    const proof: CicExpr = {
      Lam: [binderName, domainType, innerResult.proof],
    };

    return {
      proof,
      reasoning: `λ (${binderName} : ${JSON.stringify(domainType)}) => ${innerResult.reasoning}`,
    };
  }

  // Once all outer binders are peeled, solve the inner proposition
  const goalResult = solveGoal(type, ctx, depth, 0);
  if (goalResult) {
    return goalResult;
  }

  return null;
}

/**
 * Attempts backward chaining through a hypothesis function/implication type.
 * Recursively resolves domain arguments, applying De Bruijn substitutions.
 */
function tryBackchainHyp(
  headExpr: CicExpr,
  headName: string,
  fnType: CicExpr,
  targetGoal: CicExpr,
  ctx: CicHypothesis[],
  depth: number,
  searchDepth: number
): { proof: CicExpr; reasoning: string } | null {
  if (typeof fnType !== 'object' || fnType === null || !('ForallE' in fnType)) {
    return null;
  }

  const [_binderName, domain, codomain] = fnType.ForallE;

  // 1. Check direct 1-step application to reach targetGoal
  const uninstCodomain = substCicExpr(codomain, { BVar: 0 }, 0);
  if (exprEquals(uninstCodomain, targetGoal)) {
    const argProof = solveGoal(domain, ctx, depth, searchDepth + 1) || solveIntroduction(domain, ctx, depth);
    if (argProof) {
      const instCodomain = substCicExpr(codomain, argProof.proof, 0);
      if (exprEquals(instCodomain, targetGoal) || exprEquals(uninstCodomain, targetGoal)) {
        return {
          proof: { App: [headExpr, argProof.proof] },
          reasoning: `${headName} (${argProof.reasoning})`,
        };
      }
    }
  }

  // 2. Check multi-step application where codomain is another function (e.g. D1 -> D2 -> ... -> Target)
  if (typeof codomain === 'object' && codomain !== null && 'ForallE' in codomain) {
    const argProof = solveGoal(domain, ctx, depth, searchDepth + 1) || solveIntroduction(domain, ctx, depth);
    if (argProof) {
      const nextFnType = substCicExpr(codomain, argProof.proof, 0);
      const nextHeadExpr: CicExpr = { App: [headExpr, argProof.proof] };
      const nextHeadName = `${headName} (${argProof.reasoning})`;
      const rest = tryBackchainHyp(nextHeadExpr, nextHeadName, nextFnType, targetGoal, ctx, depth, searchDepth + 1);
      if (rest) {
        return rest;
      }
    }
  }

  return null;
}

/**
 * Goal Discharge Engine: Solves an atomic proposition or composite formula from the hypothesis context.
 */
function solveGoal(
  goal: CicExpr,
  ctx: CicHypothesis[],
  depth: number,
  searchDepth: number
): { proof: CicExpr; reasoning: string } | null {
  if (searchDepth > 6) return null;

  // =========================================================================
  // Rule 1: Exact Hypothesis Match
  // =========================================================================
  for (let i = 0; i < ctx.length; i++) {
    const hyp = ctx[i];
    if (exprEquals(hyp.type, goal)) {
      return {
        proof: { BVar: i },
        reasoning: `${hyp.name} (BVar(${i}))`,
      };
    }
  }

  // =========================================================================
  // Rule 2: Conjunction Introduction (And.intro)
  // =========================================================================
  const andMatch = matchAnd(goal);
  if (andMatch) {
    const { left: A, right: B } = andMatch;

    const proofA = solveIntroduction(A, ctx, depth);
    const proofB = solveIntroduction(B, ctx, depth);

    if (proofA && proofB) {
      const andIntroProof = mkApp({ Const: ['And.intro', []] }, [A, B, proofA.proof, proofB.proof]);
      return {
        proof: andIntroProof,
        reasoning: `And.intro (${proofA.reasoning}) (${proofB.reasoning})`,
      };
    }
  }

  // =========================================================================
  // Rule 3: Conjunction Projections from Hypotheses (And.left, And.right)
  // =========================================================================
  for (let i = 0; i < ctx.length; i++) {
    const hyp = ctx[i];
    const hypAnd = matchAnd(hyp.type);
    if (hypAnd) {
      const { left: A, right: B } = hypAnd;

      // Direct projection: goal is A
      if (exprEquals(A, goal)) {
        const proof = mkApp({ Const: ['And.left', []] }, [A, B, { BVar: i }]);
        return {
          proof,
          reasoning: `And.left ${hyp.name}`,
        };
      }

      // Direct projection: goal is B
      if (exprEquals(B, goal)) {
        const proof = mkApp({ Const: ['And.right', []] }, [A, B, { BVar: i }]);
        return {
          proof,
          reasoning: `And.right ${hyp.name}`,
        };
      }

      // Nested left conjunction projection: goal in A = And X Y
      const nestedLeftAnd = matchAnd(A);
      if (nestedLeftAnd) {
        const { left: X, right: Y } = nestedLeftAnd;
        const leftH = mkApp({ Const: ['And.left', []] }, [A, B, { BVar: i }]);

        if (exprEquals(X, goal)) {
          const proof = mkApp({ Const: ['And.left', []] }, [X, Y, leftH]);
          return { proof, reasoning: `And.left (And.left ${hyp.name})` };
        }
        if (exprEquals(Y, goal)) {
          const proof = mkApp({ Const: ['And.right', []] }, [X, Y, leftH]);
          return { proof, reasoning: `And.right (And.left ${hyp.name})` };
        }
      }

      // Nested right conjunction projection: goal in B = And X Y
      const nestedRightAnd = matchAnd(B);
      if (nestedRightAnd) {
        const { left: X, right: Y } = nestedRightAnd;
        const rightH = mkApp({ Const: ['And.right', []] }, [A, B, { BVar: i }]);

        if (exprEquals(X, goal)) {
          const proof = mkApp({ Const: ['And.left', []] }, [X, Y, rightH]);
          return { proof, reasoning: `And.left (And.right ${hyp.name})` };
        }
        if (exprEquals(Y, goal)) {
          const proof = mkApp({ Const: ['And.right', []] }, [X, Y, rightH]);
          return { proof, reasoning: `And.right (And.right ${hyp.name})` };
        }
      }
    }
  }

  // =========================================================================
  // Rule 4: Modus Ponens / Hypothesis Implication Application (Backchaining)
  // =========================================================================
  for (let i = 0; i < ctx.length; i++) {
    const hyp = ctx[i];
    if (hyp.type && typeof hyp.type === 'object' && 'ForallE' in hyp.type) {
      const res = tryBackchainHyp(
        { BVar: i },
        hyp.name,
        hyp.type,
        goal,
        ctx,
        depth,
        searchDepth + 1
      );
      if (res) {
        return res;
      }
    }
  }

  // =========================================================================
  // Rule 5: Disjunction Elimination (Or.elim)
  // =========================================================================
  for (let i = 0; i < ctx.length; i++) {
    const hyp = ctx[i];
    const hypOr = matchOr(hyp.type);
    if (hypOr) {
      const { left: A, right: B } = hypOr;

      // Special case: Disjunction Commutativity (Or B A from Or A B)
      const goalOr = matchOr(goal);
      if (goalOr && exprEquals(goalOr.left, B) && exprEquals(goalOr.right, A)) {
        const orElimConst: CicExpr = { Const: ['Or.elim', []] };
        const orInrConst: CicExpr = { Const: ['Or.inr', []] };
        const orInlConst: CicExpr = { Const: ['Or.inl', []] };

        const targetB = liftCicExpr(B, 1, 0);
        const targetA = liftCicExpr(A, 1, 0);

        const branchA: CicExpr = {
          Lam: [
            'a',
            A,
            mkApp(orInrConst, [targetB, targetA, { BVar: 0 }]),
          ],
        };
        const branchB: CicExpr = {
          Lam: [
            'b',
            B,
            mkApp(orInlConst, [targetB, targetA, { BVar: 0 }]),
          ],
        };

        const proof = mkApp(orElimConst, [A, B, goal, { BVar: i }, branchA, branchB]);
        return {
          proof,
          reasoning: `Or.elim ${hyp.name} (λ a => Or.inr a) (λ b => Or.inl b)`,
        };
      }
    }
  }

  // =========================================================================
  // Rule 6: Disjunction Introduction (Or.inl, Or.inr)
  // =========================================================================
  const goalOr = matchOr(goal);
  if (goalOr) {
    const { left: A, right: B } = goalOr;

    // Try Left Disjunct (Or.inl)
    const proofLeft = solveGoal(A, ctx, depth, searchDepth + 1);
    if (proofLeft) {
      const proof = mkApp({ Const: ['Or.inl', []] }, [A, B, proofLeft.proof]);
      return {
        proof,
        reasoning: `Or.inl (${proofLeft.reasoning})`,
      };
    }

    // Try Right Disjunct (Or.inr)
    const proofRight = solveGoal(B, ctx, depth, searchDepth + 1);
    if (proofRight) {
      const proof = mkApp({ Const: ['Or.inr', []] }, [A, B, proofRight.proof]);
      return {
        proof,
        reasoning: `Or.inr (${proofRight.reasoning})`,
      };
    }
  }

  // =========================================================================
  // Rule 7: Falsity Elimination / Ex Falso (False.elim)
  // =========================================================================
  for (let i = 0; i < ctx.length; i++) {
    const hyp = ctx[i];
    if (isFalse(hyp.type)) {
      const proof = mkApp({ Const: ['False.elim', []] }, [goal, { BVar: i }]);
      return {
        proof,
        reasoning: `False.elim on ${hyp.name}`,
      };
    }
  }

  return null;
}
