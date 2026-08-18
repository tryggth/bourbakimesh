/**
 * Event-Driven Telemetry & Proof Search Data Types for BourbakiMesh.
 */

export interface ProofNode {
  id: string;
  parentId: string | null;
  goal: string;
  subGoals: string[];
  tacticApplied: string | null;
  thinkingTrace?: string;
  genrmScore: number;       // S_GenRM ∈ [0, 1]
  cumulativeScore: number;   // Length-normalized path score Q(path)
  depth: number;
  status: 'open' | 'expanded' | 'proven' | 'failed' | 'pruned';
  children: string[];        // child node IDs
  timestamp: number;
}

export interface SearchConfig {
  beamWidth: number;         // Max active open nodes (default: 4)
  expansionFactorK: number;  // Candidate tactics per expansion (default: 3)
  maxDepth: number;          // Max search depth (default: 6)
  maxExpansions: number;     // Total budget cap (default: 20)
  minConfidence: number;     // GenRM pruning threshold (default: 0.15)
  lengthNormAlpha: number;   // Length penalty damping α (default: 0.6)
  reasoningBudget: number;   // Thinking tokens for Actor (default: 128)
}

export interface SearchResult {
  success: boolean;
  theoremName: string;
  rootGoal: string;
  rootId: string;
  provenPath: string[];
  tacticScript: string;
  nodesExplored: number;
  depthReached: number;
  elapsedMs: number;
  tree: Record<string, ProofNode>;
  error?: string;
}

export interface CandidateExpansion {
  tactic: string;
  thinkingTrace: string;
  genrmScore: number;
  subGoals: string[];
  closesGoal: boolean;
  pruned: boolean;
}

export interface NodeSelectedEvent {
  nodeId: string;
  goal: string;
  depth: number;
  cumulativeScore: number;
}

export interface ActorExpandedEvent {
  nodeId: string;
  goal: string;
  candidates: {
    tactic: string;
    thinkingTrace: string;
    steps: string[];
  }[];
}

export interface CriticScoredEvent {
  nodeId: string;
  candidates: CandidateExpansion[];
}

export interface TreeUpdatedEvent {
  tree: Record<string, ProofNode>;
  openQueueSize: number;
  provenCount: number;
  prunedCount: number;
  bestActiveScore: number;
}

export interface SearchCompleteEvent {
  result: SearchResult;
}

export type ProverEventMap = {
  node_selected: NodeSelectedEvent;
  actor_expanded: ActorExpandedEvent;
  critic_scored: CriticScoredEvent;
  tree_updated: TreeUpdatedEvent;
  search_complete: SearchCompleteEvent;
  search_error: { error: string };
};
