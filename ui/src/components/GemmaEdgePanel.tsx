import React, { useState, useEffect } from 'react';
import {
  Cpu,
  Brain,
  Scale,
  Zap,
  CheckCircle2,
  AlertTriangle,
  Play,
  Copy,
  Check,
  ChevronDown,
  ChevronUp,
  Activity,
  Layers,
  Gauge,
  ShieldCheck,
} from 'lucide-react';
import { gemmaEdgeController } from '../services/llmController';
import { GEMMA_4_EDGE_CONFIG } from '../config/models';
import { TacticResult, GenRmResult, LlmTelemetry } from '../workers/llm-worker';

const PRESET_GOALS = [
  {
    name: 'Mathlib.Logic.And.intro',
    goal: 'A -> B -> A ∧ B',
    hyps: ['(A : Prop)', '(B : Prop)', '(a : A)', '(b : B)'],
    candidateTactic: 'apply And.intro',
  },
  {
    name: 'Mathlib.Logic.Identity',
    goal: 'P -> P',
    hyps: ['(P : Prop)', '(h : P)'],
    candidateTactic: 'intro h; exact h',
  },
  {
    name: 'Mathlib.Logic.ModusPonens',
    goal: 'P -> (P -> Q) -> Q',
    hyps: ['(P Q : Prop)', '(hP : P)', '(hPQ : P -> Q)'],
    candidateTactic: 'intro h1 h2; apply h2; exact h1',
  },
  {
    name: 'Mathlib.Logic.OrElim',
    goal: 'A ∨ B -> (A -> C) -> (B -> C) -> C',
    hyps: ['(A B C : Prop)', '(hOr : A ∨ B)'],
    candidateTactic: 'intro hOr hA hB; cases hOr with | inl ha => exact hA ha | inr hb => exact hB hb',
  },
  {
    name: 'Mathlib.Algebra.RingEq',
    goal: '(x + y)^2 = x^2 + 2*x*y + y^2',
    hyps: ['(x y : ℤ)'],
    candidateTactic: 'ring',
  },
  {
    name: 'Mathlib.Logic.FlawedCandidate',
    goal: 'A ∧ B',
    hyps: ['(A B : Prop)'],
    candidateTactic: 'sorry',
  },
];

export const GemmaEdgePanel: React.FC = () => {
  // Controller Mode: Actor (Tactic Search) vs Critic (GenRM)
  const [controllerMode, setControllerMode] = useState<'actor' | 'critic'>('actor');
  const [thinkingBudget, setThinkingBudget] = useState<number>(256);

  // Goal & Tactic Inputs
  const [selectedPreset, setSelectedPreset] = useState<string>(PRESET_GOALS[0].name);
  const [theoremName, setTheoremName] = useState<string>(PRESET_GOALS[0].name);
  const [goalState, setGoalState] = useState<string>(PRESET_GOALS[0].goal);
  const [hypothesesText, setHypothesesText] = useState<string>(PRESET_GOALS[0].hyps.join('\n'));
  const [candidateTactic, setCandidateTactic] = useState<string>(PRESET_GOALS[0].candidateTactic);

  // Execution & Telemetry State
  const [isRunning, setIsRunning] = useState<boolean>(false);
  const [streamedText, setStreamedText] = useState<string>('');
  const [currentTokSpeed, setCurrentTokSpeed] = useState<number>(0);
  const [actorResult, setActorResult] = useState<TacticResult | null>(null);
  const [criticResult, setCriticResult] = useState<GenRmResult | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [showThinkingTrace, setShowThinkingTrace] = useState<boolean>(true);
  const [copied, setCopied] = useState<boolean>(false);

  // Model Runtime Status
  const [engineStatus, setEngineStatus] = useState<'unloaded' | 'ready' | 'running' | 'error'>('unloaded');
  const [telemetry, setTelemetry] = useState<LlmTelemetry>({
    modelId: GEMMA_4_EDGE_CONFIG.id,
    provider: 'webgpu',
    hasShaderF16: true,
    vramAllocatedMB: 1842,
    maxVramLimitMB: 4096,
    activeKvCacheSize: 512,
    slidingWindowSize: -1,
    tokensGeneratedTotal: 0,
    avgTokensPerSec: 46.2,
  });

  // Initialize engine on mount
  useEffect(() => {
    gemmaEdgeController
      .initEngine()
      .then((info) => {
        setEngineStatus('ready');
        setTelemetry((prev) => ({
          ...prev,
          provider: info.provider as any,
          hasShaderF16: info.shaderF16,
          vramAllocatedMB: info.vramAllocatedMB,
        }));
      })
      .catch((err) => {
        setEngineStatus('error');
        setErrorMessage(err.message || String(err));
      });
  }, []);

  const handleSelectPreset = (preset: typeof PRESET_GOALS[0]) => {
    setSelectedPreset(preset.name);
    setTheoremName(preset.name);
    setGoalState(preset.goal);
    setHypothesesText(preset.hyps.join('\n'));
    setCandidateTactic(preset.candidateTactic);
    setActorResult(null);
    setCriticResult(null);
    setStreamedText('');
  };

  const handleRunActor = async () => {
    setIsRunning(true);
    setErrorMessage(null);
    setStreamedText('');
    setActorResult(null);

    const hyps = hypothesesText
      .split('\n')
      .map((s) => s.trim())
      .filter(Boolean);

    try {
      const res = await gemmaEdgeController.generateTactic({
        theoremName,
        goalState,
        hypotheses: hyps,
        thinkingBudget,
        onProgress: (_token, text, speed) => {
          setStreamedText(text);
          setCurrentTokSpeed(speed);
        },
      });

      setActorResult(res);
      setCandidateTactic(res.tacticAst);
      setCurrentTokSpeed(res.tokensPerSec);

      // Refresh telemetry
      const tel = await gemmaEdgeController.getTelemetry();
      setTelemetry(tel);
    } catch (err: any) {
      setErrorMessage(err.message || String(err));
    } finally {
      setIsRunning(false);
    }
  };

  const handleRunCritic = async () => {
    setIsRunning(true);
    setErrorMessage(null);
    setCriticResult(null);

    try {
      const res = await gemmaEdgeController.evaluateCandidate({
        theoremName,
        goalState,
        candidateTactic,
      });

      setCriticResult(res);

      // Refresh telemetry
      const tel = await gemmaEdgeController.getTelemetry();
      setTelemetry(tel);
    } catch (err: any) {
      setErrorMessage(err.message || String(err));
    } finally {
      setIsRunning(false);
    }
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="flex flex-col h-full bg-slate-900 border border-slate-800 rounded-xl overflow-hidden shadow-2xl">
      {/* Top Header Banner */}
      <div className="px-6 py-4 bg-slate-950/90 border-b border-slate-800 flex flex-wrap items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <div className="p-2.5 bg-gradient-to-br from-emerald-950 to-teal-900 border border-emerald-700/60 rounded-xl text-emerald-400 shadow-lg shadow-emerald-950/40">
            <Brain className="w-5 h-5" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h2 className="text-base font-bold text-white tracking-wide">
                Gemma 4 Edge Dual-Mode WebGPU Engine
              </h2>
              <span className="px-2 py-0.5 text-[10px] font-mono font-semibold bg-emerald-950/80 text-emerald-300 border border-emerald-700/50 rounded-full">
                W4A16 / q4f16
              </span>
              <span className="px-2 py-0.5 text-[10px] font-mono font-semibold bg-blue-950/80 text-blue-300 border border-blue-700/50 rounded-full">
                {telemetry.hasShaderF16 ? 'shader-f16' : 'fp32'}
              </span>
              <span className="px-2 py-0.5 text-[10px] font-mono font-semibold bg-purple-950/80 text-purple-300 border border-purple-700/50 rounded-full">
                KV-Cache Fix (-1)
              </span>
            </div>
            <p className="text-xs text-slate-400 font-mono mt-0.5">
              Standardized edge worker: Autoregressive Tactic Search (Actor) + GenRM Logprob Verifier (Critic)
            </p>
          </div>
        </div>

        {/* Runtime Status Pill */}
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2 px-3 py-1.5 bg-slate-900/80 border border-slate-700/60 rounded-lg text-xs font-mono">
            <span
              className={`w-2 h-2 rounded-full ${
                engineStatus === 'ready' || engineStatus === 'running'
                  ? 'bg-emerald-400 animate-pulse'
                  : engineStatus === 'error'
                  ? 'bg-rose-500'
                  : 'bg-amber-400'
              }`}
            />
            <span className="text-slate-300 uppercase font-semibold">
              {engineStatus === 'ready' ? 'WebGPU Online' : engineStatus}
            </span>
            <span className="text-slate-500">|</span>
            <span className="text-slate-400">{telemetry.vramAllocatedMB} MB VRAM</span>
          </div>
        </div>
      </div>

      {/* Main Content Area */}
      <div className="p-6 overflow-y-auto space-y-6 flex-1">
        {/* Mode Selector & Thinking Budget Slider */}
        <div className="grid grid-cols-1 md:grid-cols-12 gap-4">
          {/* Mode Switch Tabs */}
          <div className="md:col-span-7 bg-slate-950/60 p-1.5 rounded-xl border border-slate-800 flex gap-2">
            <button
              type="button"
              onClick={() => setControllerMode('actor')}
              className={`flex-1 flex items-center justify-center gap-2 py-2.5 px-4 rounded-lg font-mono text-xs font-semibold transition-all ${
                controllerMode === 'actor'
                  ? 'bg-emerald-950/90 text-emerald-200 border border-emerald-700 shadow-md'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900/60'
              }`}
            >
              <Zap className="w-4 h-4 text-emerald-400" />
              <span>Actor Mode (Tactic Search)</span>
            </button>

            <button
              type="button"
              onClick={() => setControllerMode('critic')}
              className={`flex-1 flex items-center justify-center gap-2 py-2.5 px-4 rounded-lg font-mono text-xs font-semibold transition-all ${
                controllerMode === 'critic'
                  ? 'bg-blue-950/90 text-blue-200 border border-blue-700 shadow-md'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900/60'
              }`}
            >
              <Scale className="w-4 h-4 text-blue-400" />
              <span>Critic Mode (GenRM Verifier)</span>
            </button>
          </div>

          {/* Thinking Budget Slider */}
          <div className="md:col-span-5 bg-slate-950/60 p-3 rounded-xl border border-slate-800 flex flex-col justify-center">
            <div className="flex items-center justify-between mb-1.5">
              <span className="text-xs font-semibold text-slate-300 flex items-center gap-1.5">
                <Brain className="w-3.5 h-3.5 text-purple-400" />
                <span>Reasoning Token Budget</span>
              </span>
              <span className="text-xs font-mono font-bold text-purple-400 bg-purple-950/60 px-2 py-0.5 border border-purple-800 rounded">
                {controllerMode === 'actor' ? `${thinkingBudget} tokens` : '0 tokens (Critic Zero-Latency)'}
              </span>
            </div>
            <input
              type="range"
              min="0"
              max="512"
              step="32"
              disabled={controllerMode === 'critic' || isRunning}
              value={controllerMode === 'critic' ? 0 : thinkingBudget}
              onChange={(e) => setThinkingBudget(Number(e.target.value))}
              className="w-full h-1.5 bg-slate-800 rounded-lg appearance-none cursor-pointer accent-purple-500 disabled:opacity-40"
            />
          </div>
        </div>

        {/* Preset Selector Chips */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <label className="text-[11px] font-bold text-slate-400 uppercase tracking-wider">
              Curriculum Goals & Benchmarks
            </label>
            <span className="text-[11px] text-slate-500 font-mono">Select preset or edit below</span>
          </div>
          <div className="flex flex-wrap gap-2">
            {PRESET_GOALS.map((preset) => (
              <button
                key={preset.name}
                type="button"
                onClick={() => handleSelectPreset(preset)}
                className={`px-3 py-1.5 rounded-lg border text-xs font-mono transition-all ${
                  selectedPreset === preset.name
                    ? 'bg-emerald-950/80 border-emerald-700 text-emerald-300 font-semibold shadow-sm'
                    : 'bg-slate-950/60 border-slate-800 text-slate-400 hover:text-slate-200 hover:border-slate-700'
                }`}
              >
                {preset.name.split('.').pop()}
              </button>
            ))}
          </div>
        </div>

        {/* Interactive Goal Form */}
        <div className="bg-slate-950/50 p-5 rounded-xl border border-slate-800 space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="block text-xs font-semibold text-slate-300 mb-1">Theorem Name</label>
              <input
                type="text"
                value={theoremName}
                onChange={(e) => setTheoremName(e.target.value)}
                placeholder="Mathlib.Logic.And.intro"
                className="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-xs font-mono text-white focus:outline-none focus:border-emerald-500"
              />
            </div>
            <div>
              <label className="block text-xs font-semibold text-slate-300 mb-1">Goal Type / Proposition</label>
              <input
                type="text"
                value={goalState}
                onChange={(e) => setGoalState(e.target.value)}
                placeholder="A -> B -> A ∧ B"
                className="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-xs font-mono text-white focus:outline-none focus:border-emerald-500"
              />
            </div>
          </div>

          <div>
            <label className="block text-xs font-semibold text-slate-300 mb-1">Hypotheses (one per line)</label>
            <textarea
              rows={2}
              value={hypothesesText}
              onChange={(e) => setHypothesesText(e.target.value)}
              placeholder="(A : Prop)&#10;(a : A)"
              className="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-xs font-mono text-white focus:outline-none focus:border-emerald-500"
            />
          </div>

          {/* Candidate Tactic Box for Critic Mode */}
          {controllerMode === 'critic' && (
            <div className="p-3 bg-blue-950/30 border border-blue-800/40 rounded-xl space-y-2">
              <div className="flex items-center justify-between">
                <label className="block text-xs font-semibold text-blue-300">
                  Candidate Tactic to Score (GenRM Verifier)
                </label>
                <span className="text-[10px] font-mono text-blue-400">Next-Token Logprob Scoring</span>
              </div>
              <input
                type="text"
                value={candidateTactic}
                onChange={(e) => setCandidateTactic(e.target.value)}
                placeholder="apply And.intro"
                className="w-full px-3 py-2 bg-slate-950 border border-blue-900/60 rounded-lg text-xs font-mono text-blue-200 focus:outline-none focus:border-blue-500"
              />
            </div>
          )}

          {/* Action Trigger Buttons */}
          <div className="flex items-center justify-between pt-2">
            <div className="flex items-center gap-2 text-xs font-mono text-slate-400">
              <Cpu className="w-4 h-4 text-slate-500" />
              <span>Target: WebGPU Edge Worker</span>
            </div>

            {controllerMode === 'actor' ? (
              <button
                type="button"
                disabled={isRunning || !goalState}
                onClick={handleRunActor}
                className="flex items-center gap-2 px-5 py-2.5 bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 text-white text-xs font-mono font-bold rounded-lg shadow-lg shadow-emerald-950/40 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <Play className={`w-4 h-4 ${isRunning ? 'animate-spin' : ''}`} />
                <span>{isRunning ? 'Synthesizing with Thinking...' : 'Generate Tactic (Actor)'}</span>
              </button>
            ) : (
              <button
                type="button"
                disabled={isRunning || !goalState || !candidateTactic}
                onClick={handleRunCritic}
                className="flex items-center gap-2 px-5 py-2.5 bg-gradient-to-r from-blue-600 to-indigo-600 hover:from-blue-500 hover:to-indigo-500 text-white text-xs font-mono font-bold rounded-lg shadow-lg shadow-blue-950/40 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <Scale className={`w-4 h-4 ${isRunning ? 'animate-pulse' : ''}`} />
                <span>{isRunning ? 'Scoring Logprobs...' : 'Score Candidate (GenRM Critic)'}</span>
              </button>
            )}
          </div>
        </div>

        {/* Error Alert */}
        {errorMessage && (
          <div className="p-4 bg-rose-950/60 border border-rose-800 text-rose-300 text-xs font-mono rounded-xl flex items-center gap-3">
            <AlertTriangle className="w-5 h-5 text-rose-400 shrink-0" />
            <span>{errorMessage}</span>
          </div>
        )}

        {/* Telemetry Dashboard Metrics */}
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
          {/* 1. Generation Speed */}
          <div className="bg-slate-950/70 p-4 rounded-xl border border-slate-800 flex flex-col justify-between">
            <div className="flex items-center justify-between text-slate-400 mb-1">
              <span className="text-xs font-semibold">Throughput</span>
              <Activity className="w-4 h-4 text-emerald-400" />
            </div>
            <div className="text-xl font-bold font-mono text-white">
              {currentTokSpeed > 0 ? currentTokSpeed.toFixed(1) : telemetry.avgTokensPerSec.toFixed(1)}{' '}
              <span className="text-xs text-slate-400 font-normal">tok/s</span>
            </div>
            <span className="text-[10px] text-slate-500 font-mono mt-1">WebGPU W4A16 execution</span>
          </div>

          {/* 2. GenRM Logprob Score */}
          <div className="bg-slate-950/70 p-4 rounded-xl border border-slate-800 flex flex-col justify-between">
            <div className="flex items-center justify-between text-slate-400 mb-1">
              <span className="text-xs font-semibold">GenRM Score</span>
              <Gauge className="w-4 h-4 text-blue-400" />
            </div>
            <div className="text-xl font-bold font-mono text-white">
              {criticResult ? (
                <span
                  className={
                    criticResult.score > 0.7
                      ? 'text-emerald-400'
                      : criticResult.score > 0.4
                      ? 'text-amber-400'
                      : 'text-rose-400'
                  }
                >
                  {(criticResult.score * 100).toFixed(1)}%
                </span>
              ) : (
                <span className="text-slate-500">--</span>
              )}
            </div>
            <span className="text-[10px] text-slate-500 font-mono mt-1">
              {criticResult ? `p(Yes)=${(criticResult.pYes * 100).toFixed(1)}%` : 'p(Yes) / [p(Yes)+p(No)]'}
            </span>
          </div>

          {/* 3. VRAM Buffer Allocation */}
          <div className="bg-slate-950/70 p-4 rounded-xl border border-slate-800 flex flex-col justify-between">
            <div className="flex items-center justify-between text-slate-400 mb-1">
              <span className="text-xs font-semibold">VRAM Usage</span>
              <Layers className="w-4 h-4 text-purple-400" />
            </div>
            <div className="text-xl font-bold font-mono text-white">
              {telemetry.vramAllocatedMB}{' '}
              <span className="text-xs text-slate-400 font-normal">/ 4096 MB</span>
            </div>
            <div className="w-full bg-slate-800 h-1 rounded-full overflow-hidden mt-1.5">
              <div
                className="bg-purple-500 h-full"
                style={{ width: `${(telemetry.vramAllocatedMB / 4096) * 100}%` }}
              />
            </div>
          </div>

          {/* 4. Sliding Window KV-Cache Safety */}
          <div className="bg-slate-950/70 p-4 rounded-xl border border-slate-800 flex flex-col justify-between">
            <div className="flex items-center justify-between text-slate-400 mb-1">
              <span className="text-xs font-semibold">KV Cache Override</span>
              <ShieldCheck className="w-4 h-4 text-emerald-400" />
            </div>
            <div className="text-xl font-bold font-mono text-emerald-300">-1 (Active)</div>
            <span className="text-[10px] text-slate-500 font-mono mt-1">Collision workaround ok</span>
          </div>
        </div>

        {/* Actor Results Display: Intermediate Reasoning Trace & Tactic Code */}
        {(actorResult || streamedText) && controllerMode === 'actor' && (
          <div className="bg-slate-950 p-5 rounded-xl border border-emerald-900/50 space-y-4 shadow-xl">
            <div className="flex items-center justify-between border-b border-slate-800/80 pb-3">
              <div className="flex items-center gap-2">
                <CheckCircle2 className="w-4 h-4 text-emerald-400" />
                <h3 className="text-sm font-bold text-white font-mono">
                  Synthesized Lean 4 Tactic ({actorResult ? `${actorResult.elapsedMs.toFixed(0)} ms` : 'streaming...'})
                </h3>
              </div>
              <button
                type="button"
                onClick={() => setShowThinkingTrace(!showThinkingTrace)}
                className="flex items-center gap-1 text-xs text-purple-400 hover:text-purple-300 font-mono"
              >
                <span>{showThinkingTrace ? 'Hide Thinking Trace' : 'Show Thinking Trace'}</span>
                {showThinkingTrace ? <ChevronUp className="w-3.5 h-3.5" /> : <ChevronDown className="w-3.5 h-3.5" />}
              </button>
            </div>

            {/* Expandable Thinking / Reasoning Trace */}
            {showThinkingTrace && (
              <div className="p-4 bg-purple-950/20 border border-purple-900/40 rounded-lg space-y-2">
                <div className="flex items-center gap-1.5 text-xs font-semibold text-purple-300 font-mono">
                  <Brain className="w-3.5 h-3.5 text-purple-400" />
                  <span>Thinking Trace (&lt;think&gt; ... &lt;/think&gt;)</span>
                </div>
                <p className="text-xs text-slate-300 font-mono leading-relaxed bg-slate-950/60 p-3 rounded border border-purple-950">
                  {actorResult?.reasoningTrace || 'Generating step-by-step reasoning trace...'}
                </p>
              </div>
            )}

            {/* Extracted Tactic AST Output Block */}
            <div className="space-y-1.5">
              <div className="flex items-center justify-between text-xs text-slate-400 font-mono">
                <span>Extracted Tactic AST</span>
                <button
                  type="button"
                  onClick={() => copyToClipboard(actorResult?.tacticAst || candidateTactic)}
                  className="flex items-center gap-1 text-emerald-400 hover:text-emerald-300"
                >
                  {copied ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
                  <span>{copied ? 'Copied' : 'Copy'}</span>
                </button>
              </div>
              <pre className="p-3.5 bg-slate-900 border border-slate-800 rounded-lg text-emerald-300 text-sm font-mono overflow-x-auto">
                {actorResult?.tacticAst || candidateTactic || '// Tactic will appear here'}
              </pre>
            </div>
          </div>
        )}

        {/* Critic GenRM Results Display */}
        {criticResult && controllerMode === 'critic' && (
          <div className="bg-slate-950 p-5 rounded-xl border border-blue-900/50 space-y-4 shadow-xl">
            <div className="flex items-center justify-between border-b border-slate-800/80 pb-3">
              <div className="flex items-center gap-2">
                <Scale className="w-4 h-4 text-blue-400" />
                <h3 className="text-sm font-bold text-white font-mono">
                  GenRM Verifier Results ({criticResult.latencyMs.toFixed(1)} ms)
                </h3>
              </div>
              <span className="text-xs font-mono text-slate-400">
                Score: {(criticResult.score * 100).toFixed(2)}%
              </span>
            </div>

            {/* Top Token Probability Table */}
            <div className="space-y-2">
              <span className="text-xs font-semibold text-slate-300 font-mono">
                Next-Token Logprob Distribution (p(Yes) vs p(No))
              </span>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                {criticResult.topTokens.map((t) => (
                  <div
                    key={t.token}
                    className="p-3 bg-slate-900/80 border border-slate-800 rounded-lg flex items-center justify-between font-mono text-xs"
                  >
                    <div className="flex items-center gap-2">
                      <span className="font-bold text-white">Token: "{t.token}"</span>
                    </div>
                    <div className="text-right">
                      <div className="text-blue-400 font-bold">{(t.probability * 100).toFixed(2)}%</div>
                      <div className="text-[10px] text-slate-500">logprob: {t.logprob.toFixed(3)}</div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
