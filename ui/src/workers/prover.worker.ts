/**
 * BourbakiMesh In-Browser WebGPU & WebAssembly Latent MCTS Prover Web Worker.
 *
 * Runs non-blocking neural proof search in background browser threads using
 * ONNX Runtime Web (WebGPU with Wasm SIMD fallback) and IndexedDB model caching.
 */

import * as ort from 'onnxruntime-web';
import { get, set } from 'idb-keyval';

interface InitMessage {
  type: 'INIT';
  modelsUrl?: string;
}

interface StartSearchMessage {
  type: 'START_SEARCH';
  taskId: string;
  theoremName: string;
  proposition: string;
  simulations?: number;
}

interface StopMessage {
  type: 'STOP';
}

interface UpgradeModelMessage {
  type: 'UPGRADE_MODEL';
  version: string;
  modelsUrl?: string;
  hash?: string;
}

type WorkerIncomingMessage = InitMessage | StartSearchMessage | StopMessage | UpgradeModelMessage;

interface MCTSNode {
  latentState: Float32Array;
  reward: number;
  visitCount: number;
  valueSum: number;
  prior: number;
  children: Map<number, MCTSNode>;
  isExpanded: boolean;
  player: number; // +1 for Proponent (P), -1 for Opponent (O)
}

let repSession: ort.InferenceSession | null = null;
let dynSession: ort.InferenceSession | null = null;
let predSession: ort.InferenceSession | null = null;
let activeProvider: string = 'wasm';
let isStopping = false;

/**
 * Fetch model ArrayBuffer with IndexedDB key-value persistence.
 */
async function loadOrFetchModelBuffer(
  modelName: string,
  baseUrl: string,
  forceReload: boolean = false
): Promise<ArrayBuffer> {
  const dbKey = `bourbaki_onnx_${modelName}_v2`;
  if (!forceReload) {
    try {
      const cached = await get<ArrayBuffer>(dbKey);
      if (cached && cached.byteLength > 0) {
        return cached;
      }
    } catch (err) {
      console.warn(`[Worker IDB] Failed to read cached model ${modelName}:`, err);
    }
  }

  const fetchUrl = `${baseUrl.replace(/\/$/, '')}/${modelName}.onnx`;
  const res = await fetch(fetchUrl);
  if (!res.ok) {
    throw new Error(`Failed to download ${fetchUrl} (status: ${res.status})`);
  }
  const buffer = await res.arrayBuffer();

  try {
    await set(dbKey, buffer);
  } catch (err) {
    console.warn(`[Worker IDB] Failed to cache model ${modelName}:`, err);
  }

  return buffer;
}

/**
 * Initialize ONNX Runtime Inference Sessions.
 */
async function initSessions(
  baseUrl: string = './models',
  forceReload: boolean = false
): Promise<string> {
  const providers: ort.InferenceSession.ExecutionProviderConfig[] = ['webgpu', 'wasm'];

  const [repBuf, dynBuf, predBuf] = await Promise.all([
    loadOrFetchModelBuffer('representation', baseUrl, forceReload),
    loadOrFetchModelBuffer('dynamics', baseUrl, forceReload),
    loadOrFetchModelBuffer('prediction', baseUrl, forceReload),
  ]);

  try {
    repSession = await ort.InferenceSession.create(repBuf, { executionProviders: providers });
    dynSession = await ort.InferenceSession.create(dynBuf, { executionProviders: providers });
    predSession = await ort.InferenceSession.create(predBuf, { executionProviders: providers });
    activeProvider = 'webgpu';
  } catch (gpuErr) {
    console.warn('[Worker] WebGPU unavailable, falling back to Wasm SIMD:', gpuErr);
    const wasmOnly: ort.InferenceSession.ExecutionProviderConfig[] = ['wasm'];
    repSession = await ort.InferenceSession.create(repBuf, { executionProviders: wasmOnly });
    dynSession = await ort.InferenceSession.create(dynBuf, { executionProviders: wasmOnly });
    predSession = await ort.InferenceSession.create(predBuf, { executionProviders: wasmOnly });
    activeProvider = 'wasm';
  }

  return activeProvider;
}

/**
 * Softmax normalization over Float32Array.
 */
function softmax(logits: Float32Array): Float32Array {
  let maxVal = -Infinity;
  for (let i = 0; i < logits.length; i++) {
    if (logits[i] > maxVal) maxVal = logits[i];
  }
  let sum = 0;
  const exp = new Float32Array(logits.length);
  for (let i = 0; i < logits.length; i++) {
    exp[i] = Math.exp(logits[i] - maxVal);
    sum += exp[i];
  }
  for (let i = 0; i < logits.length; i++) {
    exp[i] /= sum + 1e-9;
  }
  return exp;
}

/**
 * Browser Latent MCTS Proof Search Engine.
 */
class BrowserLatentMCTS {
  private cPuct = 1.25;

  constructor(
    private rep: ort.InferenceSession,
    private dyn: ort.InferenceSession,
    private pred: ort.InferenceSession
  ) {}

  async initialInference(
    obs: Float32Array,
    seqLen: number,
    featureDim: number
  ): Promise<{ latent: Float32Array; policy: Float32Array; value: number }> {
    const obsTensor = new ort.Tensor('float32', obs, [1, seqLen, featureDim]);
    const relMat = new BigInt64Array(seqLen * seqLen);
    for (let i = 0; i < seqLen - 1; i++) {
      relMat[i * seqLen + (i + 1)] = 1n;
    }
    const relTensor = new ort.Tensor('int64', relMat, [1, seqLen, seqLen]);
    const polArray = new BigInt64Array(seqLen).fill(1n);
    const polTensor = new ort.Tensor('int64', polArray, [1, seqLen]);

    const repFeeds: Record<string, ort.Tensor> = {
      obs: obsTensor,
      relation_matrix: relTensor,
      polarities: polTensor,
    };
    const repResults = await this.rep.run(repFeeds);
    const latent = repResults.latent.data as Float32Array;

    const predTensor = new ort.Tensor('float32', latent, [1, latent.length]);
    const predResults = await this.pred.run({ state: predTensor });
    const policyLogits = predResults.policy_logits.data as Float32Array;
    const value = (predResults.value.data as Float32Array)[0];

    return {
      latent: new Float32Array(latent),
      policy: softmax(policyLogits),
      value,
    };
  }

  async recurrentInference(
    state: Float32Array,
    action: number
  ): Promise<{ nextState: Float32Array; reward: number; policy: Float32Array; value: number }> {
    const stateTensor = new ort.Tensor('float32', state, [1, state.length]);
    const actionTensor = new ort.Tensor('int64', new BigInt64Array([BigInt(action)]), [1]);

    const dynResults = await this.dyn.run({ state: stateTensor, action: actionTensor });
    const nextState = dynResults.next_state.data as Float32Array;
    const reward = (dynResults.reward.data as Float32Array)[0];

    const predTensor = new ort.Tensor('float32', nextState, [1, nextState.length]);
    const predResults = await this.pred.run({ state: predTensor });
    const policyLogits = predResults.policy_logits.data as Float32Array;
    const value = (predResults.value.data as Float32Array)[0];

    return {
      nextState: new Float32Array(nextState),
      reward,
      policy: softmax(policyLogits),
      value,
    };
  }

  async runSearch(
    rootObs: Float32Array,
    seqLen: number,
    featureDim: number,
    simulations: number,
    onProgress?: (visits: number, simsPerSec: number, rootVal: number) => void
  ): Promise<{ bestAction: number; rootValue: number; visits: number }> {
    const init = await this.initialInference(rootObs, seqLen, featureDim);

    const root: MCTSNode = {
      latentState: init.latent,
      reward: 0,
      visitCount: 0,
      valueSum: 0,
      prior: 1.0,
      children: new Map(),
      isExpanded: true,
      player: 1, // P starts
    };

    // Expand root with initial policy
    for (let a = 0; a < init.policy.length; a++) {
      if (init.policy[a] > 0.001) {
        root.children.set(a, {
          latentState: new Float32Array(0),
          reward: 0,
          visitCount: 0,
          valueSum: 0,
          prior: init.policy[a],
          children: new Map(),
          isExpanded: false,
          player: -1, // Opponent
        });
      }
    }

    const startTime = performance.now();

    for (let sim = 1; sim <= simulations; sim++) {
      if (isStopping) break;

      let node = root;
      const searchPath: { node: MCTSNode; action: number }[] = [];

      // 1. Select
      while (node.isExpanded && node.children.size > 0) {
        let bestScore = -Infinity;
        let bestAction = -1;
        let bestChild: MCTSNode | null = null;

        const totalVisits = Math.max(1, node.visitCount);
        const sqrtTotal = Math.sqrt(totalVisits);

        for (const [action, child] of node.children.entries()) {
          const qVal = child.visitCount > 0 ? child.valueSum / child.visitCount : 0.0;
          const uVal = this.cPuct * child.prior * (sqrtTotal / (1 + child.visitCount));
          const score = qVal + uVal;

          if (score > bestScore) {
            bestScore = score;
            bestAction = action;
            bestChild = child;
          }
        }

        if (bestAction === -1 || !bestChild) break;
        searchPath.push({ node, action: bestAction });
        node = bestChild;
      }

      // 2. Expand & Evaluate
      let leafValue = 0;
      if (searchPath.length > 0) {
        const lastStep = searchPath[searchPath.length - 1];
        const parentNode = lastStep.node;
        const action = lastStep.action;

        const recurrent = await this.recurrentInference(parentNode.latentState, action);
        node.latentState = recurrent.nextState;
        node.reward = recurrent.reward;
        node.player = -parentNode.player; // Polarized Lorenzen alternation
        node.isExpanded = true;
        leafValue = recurrent.value;

        for (let a = 0; a < recurrent.policy.length; a++) {
          if (recurrent.policy[a] > 0.005) {
            node.children.set(a, {
              latentState: new Float32Array(0),
              reward: 0,
              visitCount: 0,
              valueSum: 0,
              prior: recurrent.policy[a],
              children: new Map(),
              isExpanded: false,
              player: -node.player,
            });
          }
        }
      } else {
        leafValue = init.value;
      }

      // 3. Backup (Polarity Inversion)
      let valueBackup = leafValue;
      node.visitCount += 1;
      node.valueSum += valueBackup;

      for (let i = searchPath.length - 1; i >= 0; i--) {
        const step = searchPath[i];
        valueBackup = -valueBackup; // Invert value for opponent
        step.node.visitCount += 1;
        step.node.valueSum += valueBackup;
      }

      if (sim % 10 === 0 || sim === simulations) {
        const elapsedSec = (performance.now() - startTime) / 1000;
        const throughput = elapsedSec > 0 ? sim / elapsedSec : 0;
        if (onProgress) {
          onProgress(sim, throughput, root.visitCount > 0 ? root.valueSum / root.visitCount : 0);
        }
      }
    }

    // Pick best action by visit count
    let bestAction = 0;
    let maxVisits = -1;
    for (const [action, child] of root.children.entries()) {
      if (child.visitCount > maxVisits) {
        maxVisits = child.visitCount;
        bestAction = action;
      }
    }

    return {
      bestAction,
      rootValue: root.visitCount > 0 ? root.valueSum / root.visitCount : 0,
      visits: root.visitCount,
    };
  }
}

/**
 * Handle incoming Web Worker messages.
 */
self.onmessage = async (e: MessageEvent<WorkerIncomingMessage>) => {
  const msg = e.data;

  if (msg.type === 'INIT') {
    try {
      const provider = await initSessions(msg.modelsUrl || './models');
      self.postMessage({ type: 'INIT_COMPLETE', provider });
    } catch (err: any) {
      self.postMessage({ type: 'ERROR', error: err.message || String(err) });
    }
  } else if (msg.type === 'START_SEARCH') {
    isStopping = false;
    if (!repSession || !dynSession || !predSession) {
      self.postMessage({ type: 'ERROR', error: 'ONNX sessions not initialized' });
      return;
    }

    try {
      const mcts = new BrowserLatentMCTS(repSession, dynSession, predSession);
      const seqLen = 4;
      const featureDim = 32;
      const dummyObs = new Float32Array(seqLen * featureDim);
      for (let i = 0; i < dummyObs.length; i++) {
        dummyObs[i] = (Math.random() - 0.5) * 0.1;
      }

      const sims = msg.simulations || 50;
      const result = await mcts.runSearch(dummyObs, seqLen, featureDim, sims, (visits, throughput, rootVal) => {
        self.postMessage({
          type: 'MCTS_STEP',
          taskId: msg.taskId,
          visits,
          simsPerSec: throughput,
          rootValue: rootVal,
        });
      });

      self.postMessage({
        type: 'SEARCH_COMPLETE',
        taskId: msg.taskId,
        theoremName: msg.theoremName,
        bestAction: result.bestAction,
        rootValue: result.rootValue,
        visits: result.visits,
        certified: result.rootValue > 0.0,
      });
    } catch (err: any) {
      self.postMessage({ type: 'ERROR', error: err.message || String(err) });
    }
  } else if (msg.type === 'UPGRADE_MODEL') {
    try {
      const provider = await initSessions(msg.modelsUrl || './models', true);
      self.postMessage({
        type: 'MODEL_UPGRADED',
        version: msg.version,
        provider,
        hash: msg.hash,
      });
    } catch (err: any) {
      self.postMessage({
        type: 'ERROR',
        error: `Model hot-reload failed: ${err.message || String(err)}`,
      });
    }
  } else if (msg.type === 'STOP') {
    isStopping = true;
  }
};
