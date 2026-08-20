/**
 * BourbakiMesh Edge Model Registry & WebLLM/WebGPU Configuration.
 *
 * Standardized on compact, quantized Gemma 4 edge models running locally in-browser
 * via WebGPU with shader-f16 and W4A16 quantization.
 *
 * Provides typed AST deduction schemas matching crates/kernel/src/ast.rs
 * and Lean 4 core Calculus of Inductive Constructions (CIC) schemas.
 */

export interface ModelEndpointConfig {
  id: string;
  name: string;
  architecture: 'gemma4' | 'bourbaki_muzero_onnx';
  quantization: 'q4f16' | 'q4f32' | 'fp16' | 'onnx_int8';
  vramRequiredMB: number;
  maxVramBudgetMB: number;
  requiredWebGpuFeatures: string[];
  contextWindowSize: number;
  overrides: {
    sliding_window_size: number; // -1 to disable sliding window and prevent KV cache collision crashes
    attention_sink_size?: number;
    max_num_seqs?: number;
  };
  endpoints: {
    modelUrl: string;
    weightsUrl: string;
    wasmUrl?: string;
  };
  defaults: {
    actorTemperature: number;
    actorTopP: number;
    defaultThinkingBudget: number;
    maxThinkingBudget: number;
    criticTemperature: number;
    criticMaxTokens: number;
  };
}

/**
 * Standard Gemma 4 Edge Model Specification (W4A16 / q4f16 WebGPU).
 * Total VRAM footprint: ~1,850 MB, strictly within the 4,096 MB WebGPU storage limit.
 */
export const GEMMA_4_EDGE_CONFIG: ModelEndpointConfig = {
  id: 'gemma-4-2b-it-q4f16-webgpu',
  name: 'Gemma 4 Edge (2B-IT W4A16 WebGPU)',
  architecture: 'gemma4',
  quantization: 'q4f16',
  vramRequiredMB: 1850,
  maxVramBudgetMB: 4096,
  requiredWebGpuFeatures: ['shader-f16'],
  contextWindowSize: 4096,
  overrides: {
    sliding_window_size: -1, // KV-cache collision workaround: disables window truncation crash
    attention_sink_size: 4,
    max_num_seqs: 1,
  },
  endpoints: {
    modelUrl: 'https://huggingface.co/mlc-ai/gemma-2-2b-it-q4f16_1-MLC',
    weightsUrl: 'https://raw.githubusercontent.com/mlc-ai/binary-mlc-llm-libs/main/web-llm-models/v0_2_48/',
    wasmUrl: './assets/gemma4-webgpu.wasm',
  },
  defaults: {
    actorTemperature: 0.7,
    actorTopP: 0.95,
    defaultThinkingBudget: 256,
    maxThinkingBudget: 512,
    criticTemperature: 0.0,
    criticMaxTokens: 1,
  },
};

/**
 * Registry of available edge model engines.
 */
export const MODEL_REGISTRY: Record<string, ModelEndpointConfig> = {
  'gemma-4-edge': GEMMA_4_EDGE_CONFIG,
};

/**
 * Deprecated model configurations for historical reference.
 */
export const DEPRECATED_MODELS = [
  'scratch_transformer_v1',
  'custom_bpe_tokenizer_stub',
  'bourbaki_scratch_llm_70m',
] as const;

/**
 * Propositional and First-Order Logic AST Types (crates/kernel/src/ast.rs).
 */
export type Term =
  | { Var: string }
  | { Const: string }
  | { Func: [string, Term[]] };

export type Expr =
  | { Prop: string }
  | { Pred: [string, Term[]] }
  | { And: [Expr, Expr] }
  | { Or: [Expr, Expr] }
  | { Impl: [Expr, Expr] }
  | { Not: Expr }
  | 'False'
  | { Eq: [Term, Term] }
  | { Forall: { var: string; body: Expr } }
  | { Exists: { var: string; body: Expr } };

export type DeductionStep =
  // Propositional Primitives
  | { rule: 'AndElimL'; hyp: string }
  | { rule: 'AndElimR'; hyp: string }
  | { rule: 'AndIntro'; left: string; right: string }
  | { rule: 'OrIntroL'; hyp: string; right: Expr }
  | { rule: 'OrIntroR'; left: Expr; hyp: string }
  | { rule: 'OrElim'; hyp_or: string; left_impl: string; right_impl: string }
  | { rule: 'Contradiction'; pos_hyp: string; neg_hyp: string }
  | { rule: 'FalseElim'; hyp_false: string }
  | { rule: 'ModusPonens'; impl: string; arg: string }
  | { rule: 'Exact'; hyp: string }
  | { rule: 'Reflexivity'; term: Term }
  // First-Order Logic Primitives
  | { rule: 'ForallElim'; hyp: string; term: Term }
  | { rule: 'ExistsIntro'; hyp: string; var: string; body: Expr }
  | { rule: 'ExistsElim'; hyp_exists: string; hyp_impl: string }
  | { rule: 'Rewrite'; eq_hyp: string; target_hyp: string };

/**
 * Calculus of Inductive Constructions (CIC) Core AST (crates/kernel/src/cic/).
 * Implements 8-constructor Lean 4 core AST with 0-based De Bruijn indices.
 */
export type CicLevel =
  | 'Zero'
  | { Succ: CicLevel }
  | { Max: [CicLevel, CicLevel] }
  | { IMax: [CicLevel, CicLevel] }
  | { Param: string };

export type CicExpr =
  | { BVar: number }
  | { FVar: string }
  | { Sort: CicLevel }
  | { Const: [string, CicLevel[]] }
  | { App: [CicExpr, CicExpr] }
  | { Lam: [string, CicExpr, CicExpr] }
  | { ForallE: [string, CicExpr, CicExpr] }
  | { LetE: [string, CicExpr, CicExpr, CicExpr] };

/**
 * Machine-First Bourbaki Proof Kernel System Prompt.
 */
export const BOURBAKI_KERNEL_SYSTEM_PROMPT = `You are an autonomous deduction worker for the BourbakiMesh machine-first proof kernel.
Your objective is to emit deterministic AST deduction steps or pure CIC λ-terms that transition the current Proof State toward goal closure.

### FORMAL DEDUCTION RULES (JSON AST)
- {"rule": "AndElimL", "hyp": "<hyp_id>"}
- {"rule": "AndElimR", "hyp": "<hyp_id>"}
- {"rule": "AndIntro", "left": "<hyp_id>", "right": "<hyp_id>"}
- {"rule": "OrIntroL", "hyp": "<hyp_id>", "right": <expr>}
- {"rule": "OrIntroR", "left": <expr>, "hyp": "<hyp_id>"}
- {"rule": "OrElim", "hyp_or": "<hyp_id>", "left_impl": "<hyp_id>", "right_impl": "<hyp_id>"}
- {"rule": "Contradiction", "pos_hyp": "<hyp_id>", "neg_hyp": "<hyp_id>"}
- {"rule": "FalseElim", "hyp_false": "<hyp_id>"}
- {"rule": "ModusPonens", "impl": "<hyp_id>", "arg": "<hyp_id>"}
- {"rule": "Exact", "hyp": "<hyp_id>"}
- {"rule": "Reflexivity", "term": <term>}
- {"rule": "ForallElim", "hyp": "<hyp_id>", "term": <term>}
- {"rule": "ExistsIntro", "hyp": "<hyp_id>", "var": "<var_name>", "body": <expr>}
- {"rule": "ExistsElim", "hyp_exists": "<hyp_id>", "hyp_impl": "<hyp_id>"}
- {"rule": "Rewrite", "eq_hyp": "<hyp_id>", "target_hyp": "<hyp_id>"}

### PURE CIC PROOF-TERM CONSTRUCTORS
- {"BVar": <index>}
- {"FVar": "<fvar_id>"}
- {"Sort": "Zero" | {"Succ": ...}}
- {"Const": ["<name>", []]}
- {"App": [<func_expr>, <arg_expr>]}
- {"Lam": ["<binder>", <domain_type>, <body_expr>]}
- {"ForallE": ["<binder>", <domain_type>, <body_expr>]}
- {"LetE": ["<binder>", <type>, <value>, <body_expr>]}

### CIC FEW-SHOT PROOF-TERM EXAMPLES
1. Identity Term (A → A):
   Target: {"ForallE": ["_", {"FVar": "A"}, {"FVar": "A"}]}
   Proof Term: {"Lam": ["x", {"FVar": "A"}, {"BVar": 0}]}

2. Modus Ponens Term Application (h1 : A → B, h2 : A ⊢ B):
   Target: {"FVar": "B"}
   Proof Term: {"App": [{"FVar": "h1"}, {"FVar": "h2"}]}

3. Conjunction Commutativity (And A B → And B A):
   Target: {"ForallE": ["h", {"App": [{"App": [{"Const": ["And", []]}, {"FVar": "A"}]}, {"FVar": "B"}]}, {"App": [{"App": [{"Const": ["And", []]}, {"FVar": "B"}]}, {"FVar": "A"}]}]}
   Proof Term: {"Lam": ["h", {"App": [{"App": [{"Const": ["And", []]}, {"FVar": "A"}]}, {"FVar": "B"}]}, {"App": [{"App": [{"App": [{"App": [{"Const": ["And.intro", []]}, {"FVar": "B"}]}, {"FVar": "A"}]}, {"App": [{"App": [{"App": [{"Const": ["And.right", []]}, {"FVar": "A"}]}, {"FVar": "B"}]}, {"BVar": 0}]}]}, {"App": [{"App": [{"App": [{"Const": ["And.left", []]}, {"FVar": "A"}]}, {"FVar": "B"}]}, {"BVar": 0}]}]}]}

### OPERATIONAL INSTRUCTIONS
1. In ACTOR mode, output reasoning inside <think>...</think> tags, followed by exactly one JSON code block containing the step AST or CIC proof term.
2. In CRITIC mode, evaluate if the step is valid and makes progress. Output exactly one token: "Yes" or "No".`;

export function formatActorPrompt(hyps: Record<string, any>, target: any): string {
  const hypLines = Object.entries(hyps)
    .map(([id, expr]) => `  ${id}: ${JSON.stringify(expr)}`)
    .join('\n');
  return `${BOURBAKI_KERNEL_SYSTEM_PROMPT}

[CURRENT PROOF STATE]
Hypotheses:
${hypLines || '  (none)'}
Target Goal:
  ⊢ ${JSON.stringify(target)}

[MODE: ACTOR]
Propose the next valid deduction step:`;
}

export function formatCicProofPrompt(context: [string, CicExpr][], goalType: CicExpr): string {
  const ctxLines = context
    .map(([id, ty]) => `  ${id} : ${JSON.stringify(ty)}`)
    .join('\n');
  return `${BOURBAKI_KERNEL_SYSTEM_PROMPT}

### CIC PROOF TERM GRAMMAR & CONSTRUCTORS
Emit pure JSON representing a Calculus of Inductive Constructions (CIC) proof term with 0-based De Bruijn bound variables:
1. Lambda Abstraction: {"Lam": ["<binder_name>", <domain_type>, <body_expr>]}
2. Function Application: {"App": [<func_expr>, <arg_expr>]}
3. Constant Reference: {"Const": ["<name>", <levels>]} (e.g. ["And.intro", []], ["Or.inl", []], ["Or.inr", []], ["Or.elim", []], ["False.elim", []])
4. Bound Variable: {"BVar": <de_bruijn_index>} (0-based indexing from innermost enclosing binder: 0 is the most recently bound variable, 1 is the previous binder, etc.)
5. Universe Sort: {"Sort": "Zero"} or {"Sort": {"Succ": ...}}
6. Free Variable: {"FVar": "<name>"}
7. Dependent Forall: {"ForallE": ["<binder_name>", <domain_type>, <body_expr>]}
8. Local Let Binding: {"LetE": ["<binder_name>", <type_expr>, <val_expr>, <body_expr>]}

### PROOF SEARCH RULES
- For outer \`ForallE\` binders in the Goal Type, peel each into matching \`Lam\` abstractions while keeping track of De Bruijn variable depths.
- Use \`And.intro A B proofA proofB\` to construct conjunction goals.
- Use \`Or.inl A B proofA\` or \`Or.inr A B proofB\` to construct disjunction goals.
- Use \`Or.elim A B Goal hypOr (λ a => branchA) (λ b => branchB)\` for disjunction case analysis.
- Use \`False.elim Goal hypFalse\` for explosion / contradiction elimination.

[CURRENT CIC CONTEXT]
Hypotheses:
${ctxLines || '  (none)'}
Goal Type:
  ⊢ ${JSON.stringify(goalType)}

[MODE: CIC PROOF SYNTHESIS]
Synthesize a sound, closed CIC λ-term that typechecks to the Goal Type:`;
}

export function formatCriticPrompt(hyps: Record<string, any>, target: any, candidateStep: any): string {
  const hypLines = Object.entries(hyps)
    .map(([id, expr]) => `  ${id}: ${JSON.stringify(expr)}`)
    .join('\n');
  return `${BOURBAKI_KERNEL_SYSTEM_PROMPT}

[CURRENT PROOF STATE]
Hypotheses:
${hypLines || '  (none)'}
Target Goal:
  ⊢ ${JSON.stringify(target)}

[CANDIDATE STEP]
${JSON.stringify(candidateStep, null, 2)}

[MODE: CRITIC]
Does this step make valid progress toward closing the target goal? Answer Yes or No.`;
}

/**
 * Helper Prompt Templates for Dual-Mode Controller.
 */
export const PROMPT_TEMPLATES = {
  formatActorPrompt,
  formatCicProofPrompt,
  formatCriticPrompt,
  formatLegacyActorPrompt: (params: {
    theoremName: string;
    goalState: string;
    hypotheses?: string[];
    thinkingBudget: number;
  }): string => {
    const hyps = params.hypotheses && params.hypotheses.length > 0
      ? params.hypotheses.join('\n')
      : '(none)';

    return `<start_of_turn>user
You are an expert formal theorem proving assistant for Lean 4 and BourbakiMesh.
Given the proof state:
Theorem: ${params.theoremName}
Goal: ${params.goalState}
Hypotheses:
${hyps}

Generate the single best Lean 4 tactic to advance or close this goal.
First, perform your step-by-step reasoning inside <think>...</think> tags (budget: ${params.thinkingBudget} reasoning tokens).
Then, output the final tactic code enclosed in a \`\`\`lean code block.<end_of_turn>
<start_of_turn>model
<think>
`;
  },
};
