/**
 * BourbakiMesh Edge Model Registry & WebLLM/WebGPU Configuration.
 *
 * Standardized on compact, quantized Gemma 4 edge models running locally in-browser
 * via WebGPU with shader-f16 and W4A16 quantization.
 *
 * Provides typed AST deduction schemas matching crates/kernel/src/ast.rs.
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
 * Kernel AST Expression and Deduction Step Types (matching crates/kernel/src/ast.rs).
 */
export type Expr =
  | { Prop: string }
  | { And: [Expr, Expr] }
  | { Impl: [Expr, Expr] }
  | { Eq: [string, string] };

export type DeductionStep =
  | { rule: 'AndElimL'; hyp: string }
  | { rule: 'AndElimR'; hyp: string }
  | { rule: 'AndIntro'; left: string; right: string }
  | { rule: 'ModusPonens'; impl: string; arg: string }
  | { rule: 'Exact'; hyp: string }
  | { rule: 'Reflexivity'; term: string };

/**
 * Machine-First Bourbaki Proof Kernel System Prompt.
 */
export const BOURBAKI_KERNEL_SYSTEM_PROMPT = `You are an autonomous deduction worker for the BourbakiMesh machine-first proof kernel.
Your objective is to emit deterministic AST deduction steps that transition the current Proof State toward goal closure.

### FORMAL DEDUCTION RULES (JSON AST)
- {"rule": "AndElimL", "hyp": "<hyp_id>"}
- {"rule": "AndElimR", "hyp": "<hyp_id>"}
- {"rule": "AndIntro", "left": "<hyp_id>", "right": "<hyp_id>"}
- {"rule": "ModusPonens", "impl": "<hyp_id>", "arg": "<hyp_id>"}
- {"rule": "Exact", "hyp": "<hyp_id>"}
- {"rule": "Reflexivity", "term": "<expr>"}

### OPERATIONAL INSTRUCTIONS
1. Analyze hypotheses and target goal.
2. In ACTOR mode, output reasoning inside <think>...</think> tags, followed by exactly one JSON code block containing the step AST.
3. In CRITIC mode, evaluate if the step is valid and makes progress. Output exactly one token: "Yes" or "No".`;

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
  formatCriticPrompt,
  /**
   * String-based fallback prompt for legacy propositional text goals.
   */
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
