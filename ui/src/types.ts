export type Polarity = 'P' | 'O';
export type MoveKind = 'Question' | 'Assertion' | 'RootGoal';

export interface LogicalPayload {
  type: string;
  hyp_id?: number;
  premise_id?: number;
  term_repr?: string;
  branch?: string;
}

export interface DialogueMove {
  id: number;
  player: Polarity;
  kind: MoveKind;
  justification_id: number | null;
  payload: LogicalPayload;
  p_view: number[];
  o_view: number[];
}

export interface ProofBlockNode {
  id: string;
  parents: string[];
  theorem_name: string;
  proposition: string;
  extracted_term?: string;
  lean_verified: boolean;
  timestamp: number;
  status: 'certified' | 'verifying' | 'rejected';
}

export interface ProofDagData {
  nodes: ProofBlockNode[];
  edges: { source: string; target: string }[];
}

export interface DaemonStatus {
  status: string;
  active_model: string;
  peer_count: number;
  total_blocks: number;
  certified_blocks: number;
  uptime_seconds: number;
  cse_score: number;
  hardware: {
    cpu_cores: number;
    torch_device: string;
    memory_gb: number;
  };
}

export interface ModelRanking {
  name: string;
  elo: number;
  ci_95: number;
  win_rate: number;
  record: { wins: number; losses: number; draws: number };
  tier1_solve: number;
  tier2_solve: number;
  tier3_solve: number;
  sims_per_sec: number;
  cse: number;
  status: string;
}

export interface TelemetryEvent {
  type: 'mcts_step' | 'proof_attested' | 'move_added' | 'peer_connected' | 'system_status';
  timestamp: number;
  data: Record<string, unknown>;
}

export interface ProveResponse {
  success: boolean;
  theorem_name: string;
  proposition: string;
  dialogue: DialogueMove[];
  lean_code: string;
  coq_code?: string;
  isabelle_code?: string;
  dedukti_code?: string;
  verified_in_lean: boolean;
  time_ms: number;
}
