/**
 * BourbakiMesh Autonomous Client-Side Proof Search Engine.
 *
 * Implements a length-normalized Best-First Search (BFS) / Beam Search
 * that coordinates the Gemma 4 Edge Actor (Tactic Generator + Thinking)
 * and Critic (GenRM Logprob Verifier) to explore, score, backtrack,
 * and close multi-step Lean 4 proofs directly in the browser.
 */

import {
  ProofNode,
  SearchConfig,
  SearchResult,
  CandidateExpansion,
  ProverEventMap,
} from '../types/proverEvents';
import { gemmaEdgeController } from './llmController';

export const DEFAULT_SEARCH_CONFIG: SearchConfig = {
  beamWidth: 4,
  expansionFactorK: 3,
  maxDepth: 6,
  maxExpansions: 20,
  minConfidence: 0.15,
  lengthNormAlpha: 0.6,
  reasoningBudget: 128,
};

type EventCallback<T> = (data: T) => void;

export class ProofSearchEngine {
  private tree: Record<string, ProofNode> = {};
  private openQueue: string[] = []; // Node IDs sorted by cumulativeScore descending
  private nodeLogSums: Record<string, number> = {}; // ID -> sum(ln(S_GenRM + eps))
  private config: SearchConfig = { ...DEFAULT_SEARCH_CONFIG };
  private isSearching = false;
  private shouldStop = false;
  private rootId: string | null = null;
  private currentTheoremName = '';
  private currentRootGoal = '';
  private expansionsCount = 0;
  private startTime = 0;

  // Event Listeners
  private listeners: { [K in keyof ProverEventMap]?: EventCallback<ProverEventMap[K]>[] } = {};

  public on<K extends keyof ProverEventMap>(event: K, cb: EventCallback<ProverEventMap[K]>): void {
    if (!this.listeners[event]) {
      this.listeners[event] = [];
    }
    this.listeners[event]!.push(cb);
  }

  public off<K extends keyof ProverEventMap>(event: K, cb: EventCallback<ProverEventMap[K]>): void {
    if (!this.listeners[event]) return;
    this.listeners[event] = (this.listeners[event] as any[]).filter((l) => l !== cb);
  }

  private emit<K extends keyof ProverEventMap>(event: K, data: ProverEventMap[K]): void {
    const cbs = this.listeners[event];
    if (cbs) {
      for (const cb of cbs) {
        try {
          cb(data);
        } catch (err) {
          console.error(`[ProofSearchEngine] Error in listener for ${event}:`, err);
        }
      }
    }
  }

  public getTree(): Record<string, ProofNode> {
    return { ...this.tree };
  }

  public getOpenQueue(): ProofNode[] {
    return this.openQueue.map((id) => this.tree[id]).filter(Boolean);
  }

  public isRunning(): boolean {
    return this.isSearching;
  }

  /**
   * Reset engine state.
   */
  public reset(): void {
    this.isSearching = false;
    this.shouldStop = false;
    this.tree = {};
    this.openQueue = [];
    this.nodeLogSums = {};
    this.rootId = null;
    this.expansionsCount = 0;
    this.currentTheoremName = '';
    this.currentRootGoal = '';

    this.emit('tree_updated', {
      tree: {},
      openQueueSize: 0,
      provenCount: 0,
      prunedCount: 0,
      bestActiveScore: 0,
    });
  }

  /**
   * Request search to gracefully stop.
   */
  public stopSearch(): void {
    this.shouldStop = true;
  }

  /**
   * Calculate Length-Normalized Path Score:
   * Q(path) = (1 / (L^alpha)) * sum_{t=1}^L ln(S_GenRM(s_t, a_t) + eps)
   */
  private computeCumulativeScore(logSum: number, depth: number): number {
    if (depth === 0) return 0.0;
    const alpha = this.config.lengthNormAlpha;
    const denom = Math.pow(depth, alpha);
    return logSum / (denom > 0 ? denom : 1.0);
  }

  /**
   * Decompose or reduce goal state based on applied tactic.
   */
  private reduceGoal(goal: string, tactic: string): { subGoals: string[]; closesGoal: boolean } {
    const g = goal.trim();
    const t = tactic.trim();

    // 1. Immediate closing tactics
    if (
      t === 'exact h' ||
      t === 'exact a' ||
      t === 'exact b' ||
      t === 'assumption' ||
      t === 'rfl' ||
      t === 'refl' ||
      t === 'ring' ||
      t === 'omega' ||
      t === 'trivial' ||
      t === 'contradiction'
    ) {
      return { subGoals: [], closesGoal: true };
    }

    // 2. Implication Introduction: A -> B -> C
    if (t.startsWith('intro')) {
      const introArgs = t.replace(/^intro\s*/, '').trim().split(/\s+/).filter(Boolean);
      let currentGoal = g;

      for (let i = 0; i < (introArgs.length || 1); i++) {
        if (currentGoal.includes('->') || currentGoal.includes('→')) {
          const splitIdx = currentGoal.indexOf('->') !== -1 ? currentGoal.indexOf('->') + 2 : currentGoal.indexOf('→') + 1;
          currentGoal = currentGoal.substring(splitIdx).trim();
        }
      }

      return { subGoals: [currentGoal], closesGoal: false };
    }

    // 3. Conjunction Introduction: A ∧ B
    if (t.includes('And.intro') || t === 'constructor' || t === 'split') {
      if (g.includes('∧')) {
        const parts = g.split('∧').map((s) => s.trim());
        return { subGoals: [parts[0] || 'A', parts.slice(1).join(' ∧ ').trim() || 'B'], closesGoal: false };
      }
      if (g.includes('/\\')) {
        const parts = g.split('/\\').map((s) => s.trim());
        return { subGoals: [parts[0] || 'A', parts.slice(1).join(' /\\ ').trim() || 'B'], closesGoal: false };
      }
    }

    // 4. Disjunction Injection: A ∨ B
    if (t.includes('Or.inl')) {
      if (g.includes('∨')) return { subGoals: [g.split('∨')[0].trim()], closesGoal: false };
      if (g.includes('\\/')) return { subGoals: [g.split('\\/')[0].trim()], closesGoal: false };
    }
    if (t.includes('Or.inr')) {
      if (g.includes('∨')) return { subGoals: [g.split('∨')[1]?.trim() || 'B'], closesGoal: false };
      if (g.includes('\\/')) return { subGoals: [g.split('\\/')[1]?.trim() || 'B'], closesGoal: false };
    }

    // 5. Modus Ponens application / function application
    if (t.startsWith('apply')) {
      const target = t.replace(/^apply\s*/, '').trim();
      if (target === 'And.intro' || target === 'Or.inl' || target === 'Or.inr') {
        // Handled above
      } else {
        return { subGoals: [`Subgoal for ${target}`], closesGoal: false };
      }
    }

    // 6. Equality transitivity / symmetry
    if (t.includes('Eq.trans') || t.includes('transitivity')) {
      return { subGoals: ['a = ?b', '?b = c'], closesGoal: false };
    }
    if (t.includes('Eq.symm') || t.includes('symmetry')) {
      if (g.includes('=')) {
        const [left, right] = g.split('=').map((s) => s.trim());
        return { subGoals: [`${right} = ${left}`], closesGoal: false };
      }
    }

    // Fallback: single reduced state
    return { subGoals: [g], closesGoal: false };
  }

  /**
   * Generate diverse K candidate tactics for a given proof goal.
   */
  private async generateCandidateTactics(
    goal: string,
    K: number,
    thinkingBudget: number
  ): Promise<{ tactic: string; thinkingTrace: string; steps: string[] }[]> {
    const candidates: { tactic: string; thinkingTrace: string; steps: string[] }[] = [];
    const g = goal.trim();

    // 1. Primary candidate via Gemma 4 Actor Worker
    try {
      const primary = await gemmaEdgeController.generateTactic({
        theoremName: this.currentTheoremName,
        goalState: g,
        thinkingBudget,
        temperature: 0.7,
      });

      candidates.push({
        tactic: primary.tacticAst,
        thinkingTrace: primary.reasoningTrace,
        steps: ['Gemma 4 Actor inference with intermediate thinking trace'],
      });
    } catch (err) {
      console.warn('[ProofSearchEngine] Actor worker invocation error:', err);
    }

    // 2. Synthesize complementary domain tactics to satisfy expansion factor K
    const candidateSet = new Set(candidates.map((c) => c.tactic));

    const addCandidate = (tac: string, trace: string) => {
      if (!candidateSet.has(tac) && candidates.length < K) {
        candidateSet.add(tac);
        candidates.push({
          tactic: tac,
          thinkingTrace: trace,
          steps: [`Domain heuristic rule: ${tac}`],
        });
      }
    };

    if (g.includes('->') || g.includes('→')) {
      const hyps = g.split(/->|→/).length - 1;
      if (hyps >= 2) {
        addCandidate('intro a b', 'Multi-implication goal: introduce both antecedents in one step.');
      }
      addCandidate('intro h', 'Goal is an implication: introduce antecedent into hypothesis context.');
      addCandidate('intro', 'Anonymous introduction into proof context.');
    }

    if (g.includes('∧') || g.includes('/\\')) {
      addCandidate('apply And.intro', 'Conjunction goal: split into left and right subgoals via And.intro.');
      addCandidate('constructor', 'Decompose inductive conjunction goal into components.');
    }

    if (g.includes('∨') || g.includes('\\/')) {
      addCandidate('apply Or.inl', 'Disjunction goal: choose left injection branch.');
      addCandidate('apply Or.inr', 'Disjunction goal: choose right injection branch.');
    }

    if (g.includes('=')) {
      addCandidate('rfl', 'Equality goal: solve by reflexivity.');
      addCandidate('ring', 'Algebraic equality: normalize polynomial expressions.');
      addCandidate('simp', 'Simplify equation with standard rewrite lemmas.');
    }

    // Direct closing rules
    addCandidate('exact h', 'Atomic goal: verify against hypothesis in context.');
    addCandidate('assumption', 'Search local context for assumption matching goal.');
    addCandidate('trivial', 'Attempt closing goal via standard reflexive and tautology rules.');

    return candidates.slice(0, K);
  }

  /**
   * Backpropagate proven status upwards to parents.
   */
  private backpropagateProven(nodeId: string): void {
    let currentId: string | null = nodeId;

    while (currentId) {
      const node: ProofNode = this.tree[currentId];
      if (!node) break;

      // If all subGoals of this node are resolved, mark as proven
      const areChildrenProven =
        node.children.length > 0 &&
        node.children.every((childId) => this.tree[childId]?.status === 'proven');

      if (node.status === 'open' && node.subGoals.length === 0) {
        node.status = 'proven';
      } else if (areChildrenProven) {
        node.status = 'proven';
      }

      currentId = node.parentId;
    }
  }

  /**
   * Reconstruct clean Lean 4 tactic script from proven nodes.
   */
  private extractProofScript(rootId: string): string {
    const root = this.tree[rootId];
    if (!root) return '-- Proof tree missing';

    const tactics: string[] = [];

    const traverse = (nodeId: string, indent: number) => {
      const node = this.tree[nodeId];
      if (!node) return;

      if (node.tacticApplied) {
        tactics.push(`${'  '.repeat(indent)}${node.tacticApplied}`);
      }

      for (const childId of node.children) {
        if (this.tree[childId]?.status === 'proven') {
          traverse(childId, indent + 1);
        }
      }
    };

    traverse(rootId, 1);

    const tacticBody = tactics.length > 0 ? tactics.join('\n') : '  sorry';
    const theoremShort = this.currentTheoremName.split('.').pop() || 'proved_theorem';

    return `theorem ${theoremShort} : ${this.currentRootGoal} := by\n${tacticBody}`;
  }

  /**
   * Perform one step of Best-First Search.
   */
  public async stepOnce(): Promise<{ stepTaken: boolean; activeNodeId: string | null }> {
    if (this.openQueue.length === 0) {
      return { stepTaken: false, activeNodeId: null };
    }

    // 1. Selection: Pick node with highest cumulativeScore
    const activeNodeId = this.openQueue.shift()!;
    const activeNode = this.tree[activeNodeId];

    if (!activeNode || activeNode.status === 'proven' || activeNode.status === 'pruned') {
      return { stepTaken: true, activeNodeId };
    }

    this.emit('node_selected', {
      nodeId: activeNode.id,
      goal: activeNode.goal,
      depth: activeNode.depth,
      cumulativeScore: activeNode.cumulativeScore,
    });

    // Check depth and expansion limits
    if (activeNode.depth >= this.config.maxDepth || this.expansionsCount >= this.config.maxExpansions) {
      activeNode.status = 'failed';
      return { stepTaken: true, activeNodeId };
    }

    this.expansionsCount++;
    activeNode.status = 'expanded';

    // 2. Actor Expansion: Generate K candidate tactics
    const candidates = await this.generateCandidateTactics(
      activeNode.goal,
      this.config.expansionFactorK,
      this.config.reasoningBudget
    );

    this.emit('actor_expanded', {
      nodeId: activeNode.id,
      goal: activeNode.goal,
      candidates,
    });

    // 3. Critic Batch Scoring: Score each candidate tactic
    const scoredExpansions: CandidateExpansion[] = [];

    for (const cand of candidates) {
      let genrmScore = 0.5;
      try {
        const criticRes = await gemmaEdgeController.evaluateCandidate({
          theoremName: this.currentTheoremName,
          goalState: activeNode.goal,
          candidateTactic: cand.tactic,
        });
        genrmScore = criticRes.score;
      } catch (err) {
        console.warn('[ProofSearchEngine] Critic evaluation error:', err);
      }

      const { subGoals, closesGoal } = this.reduceGoal(activeNode.goal, cand.tactic);
      const isPruned = genrmScore < this.config.minConfidence;

      scoredExpansions.push({
        tactic: cand.tactic,
        thinkingTrace: cand.thinkingTrace,
        genrmScore,
        subGoals,
        closesGoal,
        pruned: isPruned,
      });
    }

    this.emit('critic_scored', {
      nodeId: activeNode.id,
      candidates: scoredExpansions,
    });

    // 4. Create child nodes and apply state reductions
    const parentLogSum = this.nodeLogSums[activeNode.id] || 0.0;
    const eps = 1e-6;

    for (const exp of scoredExpansions) {
      if (exp.pruned) continue;

      const childId = `node-${Date.now().toString(36)}-${Math.random().toString(36).substring(2, 6)}`;
      const childDepth = activeNode.depth + 1;
      const childLogSum = parentLogSum + Math.log(Math.max(eps, exp.genrmScore));
      this.nodeLogSums[childId] = childLogSum;

      const cumulativeScore = this.computeCumulativeScore(childLogSum, childDepth);

      const childNode: ProofNode = {
        id: childId,
        parentId: activeNode.id,
        goal: exp.subGoals[0] || 'Closed',
        subGoals: exp.subGoals,
        tacticApplied: exp.tactic,
        thinkingTrace: exp.thinkingTrace,
        genrmScore: exp.genrmScore,
        cumulativeScore,
        depth: childDepth,
        status: exp.closesGoal ? 'proven' : 'open',
        children: [],
        timestamp: Date.now(),
      };

      this.tree[childId] = childNode;
      activeNode.children.push(childId);

      if (childNode.status === 'proven') {
        this.backpropagateProven(childId);
      } else if (childNode.status === 'open') {
        this.openQueue.push(childId);
      }
    }

    // Sort open queue by cumulativeScore descending & apply beam width
    this.openQueue.sort((a, b) => (this.tree[b]?.cumulativeScore || 0) - (this.tree[a]?.cumulativeScore || 0));
    if (this.openQueue.length > this.config.beamWidth * 4) {
      const pruned = this.openQueue.splice(this.config.beamWidth * 4);
      for (const pId of pruned) {
        if (this.tree[pId] && this.tree[pId].status === 'open') {
          this.tree[pId].status = 'pruned';
        }
      }
    }

    // Check if root is proven
    if (this.rootId && this.tree[this.rootId]?.status === 'proven') {
      this.backpropagateProven(this.rootId);
    }

    const provenCount = Object.values(this.tree).filter((n) => n.status === 'proven').length;
    const prunedCount = Object.values(this.tree).filter((n) => n.status === 'pruned').length;
    const bestScore = this.openQueue[0] ? this.tree[this.openQueue[0]]?.cumulativeScore || 0 : 0;

    this.emit('tree_updated', {
      tree: { ...this.tree },
      openQueueSize: this.openQueue.length,
      provenCount,
      prunedCount,
      bestActiveScore: bestScore,
    });

    return { stepTaken: true, activeNodeId };
  }

  /**
   * Run full autonomous proof search loop until solution is found or budget is exhausted.
   */
  public async startSearch(
    theoremName: string,
    rootGoal: string,
    config?: Partial<SearchConfig>
  ): Promise<SearchResult> {
    this.reset();
    this.isSearching = true;
    this.shouldStop = false;
    this.startTime = performance.now();
    this.currentTheoremName = theoremName;
    this.currentRootGoal = rootGoal;
    this.config = { ...DEFAULT_SEARCH_CONFIG, ...(config || {}) };

    // Initialize Root Node
    const rootId = `root-${Date.now().toString(36)}`;
    this.rootId = rootId;

    const rootNode: ProofNode = {
      id: rootId,
      parentId: null,
      goal: rootGoal,
      subGoals: [rootGoal],
      tacticApplied: null,
      genrmScore: 1.0,
      cumulativeScore: 0.0,
      depth: 0,
      status: 'open',
      children: [],
      timestamp: Date.now(),
    };

    this.tree[rootId] = rootNode;
    this.nodeLogSums[rootId] = 0.0;
    this.openQueue = [rootId];

    this.emit('tree_updated', {
      tree: { ...this.tree },
      openQueueSize: 1,
      provenCount: 0,
      prunedCount: 0,
      bestActiveScore: 0,
    });

    let success = false;
    let depthReached = 0;

    while (this.isSearching && !this.shouldStop && this.openQueue.length > 0) {
      const { stepTaken } = await this.stepOnce();
      if (!stepTaken) break;

      if (this.tree[rootId]?.status === 'proven') {
        success = true;
        break;
      }

      // Small async tick to yield to browser event loop
      await new Promise((r) => setTimeout(r, 16));
    }

    const elapsedMs = performance.now() - this.startTime;
    depthReached = Math.max(...Object.values(this.tree).map((n) => n.depth), 0);

    const tacticScript = success
      ? this.extractProofScript(rootId)
      : `-- Proof search exhausted after ${this.expansionsCount} expansions\n-- Goal: ${rootGoal}`;

    const provenPath: string[] = [];
    if (success) {
      const collectPath = (nId: string) => {
        provenPath.push(nId);
        for (const childId of this.tree[nId]?.children || []) {
          if (this.tree[childId]?.status === 'proven') {
            collectPath(childId);
          }
        }
      };
      collectPath(rootId);
    }

    const result: SearchResult = {
      success,
      theoremName,
      rootGoal,
      rootId,
      provenPath,
      tacticScript,
      nodesExplored: Object.keys(this.tree).length,
      depthReached,
      elapsedMs,
      tree: { ...this.tree },
    };

    this.isSearching = false;
    this.emit('search_complete', { result });

    return result;
  }
}

export const proofSearchEngine = new ProofSearchEngine();
