/**
 * BourbakiMesh Gemma 4 Edge LLM Controller Service.
 *
 * Provides a clean, typed interface for UI and search engine components to interact with
 * the Gemma 4 WebGPU worker in Actor Mode (Tactic Search + Thinking + JSON AST emission)
 * and Critic Mode (GenRM Logprob Verifier).
 */

import {
  TacticResult,
  GenRmResult,
  LlmTelemetry,
  LlmWorkerIncomingMessage,
} from '../workers/llm-worker';
import { GEMMA_4_EDGE_CONFIG, DeductionStep, Expr } from '../config/models';

class GemmaEdgeController {
  private worker: Worker | null = null;
  private pendingRequests: Map<string, { resolve: (val: any) => void; reject: (err: any) => void }> = new Map();
  private progressCallbacks: Map<string, (token: string, currentText: string, tokSpeed: number) => void> = new Map();
  private isInitializing = false;
  private initPromise: Promise<{ provider: string; shaderF16: boolean; vramAllocatedMB: number }> | null = null;

  public get initializing(): boolean {
    return this.isInitializing;
  }

  private getWorker(): Worker {
    if (!this.worker) {
      this.worker = new Worker(new URL('../workers/llm-worker.ts', import.meta.url), {
        type: 'module',
      });

      this.worker.onmessage = (e: MessageEvent<any>) => {
        const msg = e.data;

        if (msg.type === 'INIT_LLM_COMPLETE') {
          const req = this.pendingRequests.get('init');
          if (req) {
            req.resolve({
              provider: msg.provider,
              shaderF16: msg.shaderF16,
              vramAllocatedMB: msg.vramAllocatedMB,
            });
            this.pendingRequests.delete('init');
          }
        } else if (msg.type === 'TACTIC_PROGRESS') {
          const cb = this.progressCallbacks.get(msg.taskId);
          if (cb) {
            cb(msg.token, msg.currentText, msg.tokensPerSec);
          }
        } else if (msg.type === 'TACTIC_COMPLETE') {
          const req = this.pendingRequests.get(msg.taskId);
          if (req) {
            req.resolve(msg as TacticResult);
            this.pendingRequests.delete(msg.taskId);
            this.progressCallbacks.delete(msg.taskId);
          }
        } else if (msg.type === 'GENRM_COMPLETE') {
          const req = this.pendingRequests.get(msg.taskId);
          if (req) {
            req.resolve(msg as GenRmResult);
            this.pendingRequests.delete(msg.taskId);
          }
        } else if (msg.type === 'TELEMETRY_RESPONSE' || msg.type === 'TELEMETRY_DATA') {
          const req = this.pendingRequests.get('telemetry');
          if (req) {
            req.resolve(msg.telemetry as LlmTelemetry);
            this.pendingRequests.delete('telemetry');
          }
        } else if (msg.type === 'WORKER_ERROR' || msg.type === 'ERROR') {
          console.error('[GemmaEdgeController] Worker error:', msg.error);
          for (const [key, req] of this.pendingRequests.entries()) {
            req.reject(new Error(msg.error));
            this.pendingRequests.delete(key);
          }
        }
      };
    }
    return this.worker;
  }

  /**
   * Initialize the Gemma 4 WebGPU edge model runtime.
   */
  async initEngine(modelId: string = GEMMA_4_EDGE_CONFIG.id): Promise<{
    provider: string;
    shaderF16: boolean;
    vramAllocatedMB: number;
  }> {
    if (this.initPromise) return this.initPromise;

    this.isInitializing = true;
    this.initPromise = new Promise((resolve, reject) => {
      const worker = this.getWorker();
      this.pendingRequests.set('init', { resolve, reject });
      worker.postMessage({
        type: 'INIT_LLM',
        modelId,
      } as LlmWorkerIncomingMessage);
    });

    return this.initPromise;
  }

  /**
   * Actor Mode: Generate a Lean 4 tactic / AST deduction step with reasoning trace inside <think>...</think>.
   */
  async generateTactic(params: {
    theoremName?: string;
    goalState?: string;
    hyps?: Record<string, Expr>;
    target?: Expr;
    hypotheses?: string[];
    thinkingBudget?: number;
    maxTokens?: number;
    temperature?: number;
    onProgress?: (token: string, currentText: string, speedTokSec: number) => void;
  }): Promise<TacticResult> {
    await this.initEngine();
    const taskId = `actor-${Date.now().toString(36)}-${Math.random().toString(36).substring(2, 6)}`;

    if (params.onProgress) {
      this.progressCallbacks.set(taskId, params.onProgress);
    }

    return new Promise((resolve, reject) => {
      const worker = this.getWorker();
      this.pendingRequests.set(taskId, { resolve, reject });
      worker.postMessage({
        type: 'GENERATE_TACTIC',
        taskId,
        theoremName: params.theoremName || 'kernel_theorem',
        goalState: params.goalState || '',
        hyps: params.hyps,
        target: params.target,
        hypotheses: params.hypotheses,
        thinkingBudget: params.thinkingBudget ?? GEMMA_4_EDGE_CONFIG.defaults.defaultThinkingBudget,
        maxTokens: params.maxTokens ?? 512,
        temperature: params.temperature ?? GEMMA_4_EDGE_CONFIG.defaults.actorTemperature,
      } as LlmWorkerIncomingMessage);
    });
  }

  /**
   * Critic Mode (GenRM Verifier): Evaluate candidate tactic / AST progress via next-token logprob scoring.
   * Computes S_GenRM = p(Yes) / (p(Yes) + p(No)).
   */
  async evaluateCandidate(params: {
    goalState?: string;
    candidateTactic?: string;
    hyps?: Record<string, Expr>;
    target?: Expr;
    candidateStep?: DeductionStep;
    theoremName?: string;
  }): Promise<GenRmResult> {
    await this.initEngine();
    const taskId = `critic-${Date.now().toString(36)}-${Math.random().toString(36).substring(2, 6)}`;

    return new Promise((resolve, reject) => {
      const worker = this.getWorker();
      this.pendingRequests.set(taskId, { resolve, reject });
      worker.postMessage({
        type: 'EVALUATE_CANDIDATE',
        taskId,
        theoremName: params.theoremName,
        goalState: params.goalState || '',
        hyps: params.hyps,
        target: params.target,
        candidateStep: params.candidateStep,
        candidateTactic: params.candidateTactic || '',
      } as LlmWorkerIncomingMessage);
    });
  }

  /**
   * Fetch real-time WebGPU VRAM buffer telemetry and throughput stats.
   */
  async getTelemetry(): Promise<LlmTelemetry> {
    await this.initEngine();
    return new Promise((resolve, reject) => {
      const worker = this.getWorker();
      this.pendingRequests.set('telemetry', { resolve, reject });
      worker.postMessage({ type: 'GET_TELEMETRY' } as LlmWorkerIncomingMessage);
    });
  }

  /**
   * Terminate worker instance.
   */
  terminate(): void {
    if (this.worker) {
      this.worker.terminate();
      this.worker = null;
    }
    this.pendingRequests.clear();
    this.progressCallbacks.clear();
    this.initPromise = null;
    this.isInitializing = false;
  }
}

export const gemmaEdgeController = new GemmaEdgeController();
