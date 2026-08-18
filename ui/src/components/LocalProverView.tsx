import React, { useState, useEffect } from 'react';
import {
  Play,
  StepForward,
  Square,
  RotateCcw,
  CheckCircle2,
  Brain,
  Sliders,
  Copy,
  Check,
  ChevronRight,
  Activity,
  Layers,
  Search,
  Workflow,
} from 'lucide-react';
import {
  ProofNode,
  SearchConfig,
  SearchResult,
  TreeUpdatedEvent,
} from '../types/proverEvents';
import { proofSearchEngine, DEFAULT_SEARCH_CONFIG } from '../services/proofSearchEngine';
import { gemmaEdgeController } from '../services/llmController';

const BENCHMARK_PRESETS = [
  {
    name: 'Mathlib.Logic.And.intro',
    goal: 'A -> B -> A ∧ B',
    desc: 'Propositional conjunction introduction',
  },
  {
    name: 'Mathlib.Logic.And.comm',
    goal: 'A ∧ B -> B ∧ A',
    desc: 'Conjunction commutativity via decomposition',
  },
  {
    name: 'Mathlib.Logic.DeMorgan',
    goal: '¬(A ∨ B) -> ¬A ∧ ¬B',
    desc: "De Morgan's law over disjunction negation",
  },
  {
    name: 'Mathlib.Logic.ModusPonensChain',
    goal: 'P -> (P -> Q) -> (Q -> R) -> R',
    desc: 'Multi-step implication resolution chain',
  },
  {
    name: 'Mathlib.Algebra.EqTrans',
    goal: '(a = b) -> (b = c) -> (a = c)',
    desc: 'Equational reasoning transitivity',
  },
  {
    name: 'Mathlib.Logic.DoubleNegIntro',
    goal: 'P -> ¬¬P',
    desc: 'Intuitionistic double negation introduction',
  },
];

export const LocalProverView: React.FC = () => {
  // Goal & Preset State
  const [selectedPreset, setSelectedPreset] = useState<string>(BENCHMARK_PRESETS[0].name);
  const [theoremName, setTheoremName] = useState<string>(BENCHMARK_PRESETS[0].name);
  const [rootGoal, setRootGoal] = useState<string>(BENCHMARK_PRESETS[0].goal);

  // Search Engine Configuration
  const [config, setConfig] = useState<SearchConfig>({ ...DEFAULT_SEARCH_CONFIG });
  const [showConfig, setShowConfig] = useState<boolean>(false);

  // Search State & Tree Data
  const [isSearching, setIsSearching] = useState<boolean>(false);
  const [tree, setTree] = useState<Record<string, ProofNode>>({});
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [searchResult, setSearchResult] = useState<SearchResult | null>(null);
  const [copiedScript, setCopiedScript] = useState<boolean>(false);

  // Telemetry Metrics
  const [openCount, setOpenCount] = useState<number>(0);
  const [provenCount, setProvenCount] = useState<number>(0);
  const [prunedCount, setPrunedCount] = useState<number>(0);
  const [activeStepGoal, setActiveStepGoal] = useState<string>('');
  const [vramMB, setVramMB] = useState<number>(1842);
  const [tokSpeed, setTokSpeed] = useState<number>(46.2);

  // Subscribe to Search Engine Events
  useEffect(() => {
    const handleTreeUpdated = (e: TreeUpdatedEvent) => {
      setTree(e.tree);
      setOpenCount(e.openQueueSize);
      setProvenCount(e.provenCount);
      setPrunedCount(e.prunedCount);
    };

    const handleNodeSelected = (e: { nodeId: string; goal: string }) => {
      setActiveStepGoal(e.goal);
      setSelectedNodeId(e.nodeId);
    };

    const handleSearchComplete = (e: { result: SearchResult }) => {
      setSearchResult(e.result);
      setIsSearching(false);
      if (e.result.rootId) {
        setSelectedNodeId(e.result.rootId);
      }
    };

    proofSearchEngine.on('tree_updated', handleTreeUpdated);
    proofSearchEngine.on('node_selected', handleNodeSelected);
    proofSearchEngine.on('search_complete', handleSearchComplete);

    // Initial Engine Telemetry Check
    gemmaEdgeController.getTelemetry().then((tel) => {
      setVramMB(tel.vramAllocatedMB);
      setTokSpeed(tel.avgTokensPerSec || 46.2);
    });

    return () => {
      proofSearchEngine.off('tree_updated', handleTreeUpdated);
      proofSearchEngine.off('node_selected', handleNodeSelected);
      proofSearchEngine.off('search_complete', handleSearchComplete);
    };
  }, []);

  const handleSelectPreset = (preset: typeof BENCHMARK_PRESETS[0]) => {
    setSelectedPreset(preset.name);
    setTheoremName(preset.name);
    setRootGoal(preset.goal);
    handleReset();
  };

  const handleStartSearch = async () => {
    if (!rootGoal) return;
    setIsSearching(true);
    setSearchResult(null);
    try {
      const res = await proofSearchEngine.startSearch(theoremName, rootGoal, config);
      setSearchResult(res);
    } catch (err) {
      console.error('Proof search error:', err);
    } finally {
      setIsSearching(false);
    }
  };

  const handleStepOnce = async () => {
    if (Object.keys(tree).length === 0) {
      // Initialize tree first
      proofSearchEngine.startSearch(theoremName, rootGoal, config).catch(console.error);
      return;
    }
    await proofSearchEngine.stepOnce();
  };

  const handleStopSearch = () => {
    proofSearchEngine.stopSearch();
    setIsSearching(false);
  };

  const handleReset = () => {
    proofSearchEngine.reset();
    setTree({});
    setSelectedNodeId(null);
    setSearchResult(null);
    setOpenCount(0);
    setProvenCount(0);
    setPrunedCount(0);
    setActiveStepGoal('');
  };

  const copyProofScript = (script: string) => {
    navigator.clipboard.writeText(script);
    setCopiedScript(true);
    setTimeout(() => setCopiedScript(false), 2000);
  };

  const selectedNode = selectedNodeId ? tree[selectedNodeId] : null;
  const nodesList = Object.values(tree);

  return (
    <div className="flex flex-col h-full bg-slate-900 border border-slate-800 rounded-xl overflow-hidden shadow-2xl">
      {/* Top Header Banner */}
      <div className="px-6 py-4 bg-slate-950/90 border-b border-slate-800 flex flex-wrap items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <div className="p-2.5 bg-gradient-to-br from-blue-950 to-indigo-900 border border-blue-700/60 rounded-xl text-blue-400 shadow-lg shadow-blue-950/40">
            <Workflow className="w-5 h-5" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h2 className="text-base font-bold text-white tracking-wide">
                Autonomous Proof-Search Engine (Client BFS)
              </h2>
              <span className="px-2 py-0.5 text-[10px] font-mono font-semibold bg-emerald-950/80 text-emerald-300 border border-emerald-700/50 rounded-full">
                Length-Normalized BFS
              </span>
              <span className="px-2 py-0.5 text-[10px] font-mono font-semibold bg-blue-950/80 text-blue-300 border border-blue-700/50 rounded-full">
                Gemma 4 WebGPU
              </span>
            </div>
            <p className="text-xs text-slate-400 font-mono mt-0.5">
              Coordinates Actor (Thinking Tactic Generator) & Critic (GenRM Verifier) to explore and close proof trees
            </p>
          </div>
        </div>

        {/* Real-time Status Badges */}
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2 px-3 py-1.5 bg-slate-900/80 border border-slate-700/60 rounded-lg text-xs font-mono">
            <span
              className={`w-2 h-2 rounded-full ${
                isSearching ? 'bg-emerald-400 animate-ping' : searchResult?.success ? 'bg-emerald-400' : 'bg-slate-500'
              }`}
            />
            <span className="text-slate-300 uppercase font-semibold">
              {isSearching ? 'Searching...' : searchResult?.success ? 'Proven!' : 'Idle'}
            </span>
            <span className="text-slate-500">|</span>
            <span className="text-slate-400">{vramMB} MB VRAM</span>
          </div>
        </div>
      </div>

      {/* Main Container */}
      <div className="flex-1 overflow-y-auto p-6 space-y-6">
        {/* Preset Selector Chips */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <label className="text-[11px] font-bold text-slate-400 uppercase tracking-wider">
              Curriculum Logic Goals & Tactic Benchmarks
            </label>
            <span className="text-[11px] text-slate-500 font-mono">Select benchmark preset</span>
          </div>
          <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-2">
            {BENCHMARK_PRESETS.map((preset) => (
              <button
                key={preset.name}
                type="button"
                onClick={() => handleSelectPreset(preset)}
                className={`p-2.5 rounded-lg border text-left transition-all ${
                  selectedPreset === preset.name
                    ? 'bg-blue-950/80 border-blue-600 text-blue-200 shadow-md'
                    : 'bg-slate-950/50 border-slate-800 text-slate-400 hover:text-slate-200 hover:border-slate-700'
                }`}
              >
                <div className="text-xs font-mono font-bold truncate">{preset.name.split('.').pop()}</div>
                <div className="text-[10px] text-slate-500 font-mono truncate mt-0.5">{preset.goal}</div>
              </button>
            ))}
          </div>
        </div>

        {/* Goal Input & Search Controls */}
        <div className="bg-slate-950/50 p-5 rounded-xl border border-slate-800 space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-12 gap-4">
            <div className="md:col-span-4">
              <label className="block text-xs font-semibold text-slate-300 mb-1 font-mono">Theorem Name</label>
              <input
                type="text"
                value={theoremName}
                onChange={(e) => setTheoremName(e.target.value)}
                placeholder="Mathlib.Logic.And.intro"
                className="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-xs font-mono text-white focus:outline-none focus:border-blue-500"
              />
            </div>
            <div className="md:col-span-8">
              <label className="block text-xs font-semibold text-slate-300 mb-1 font-mono">Root Goal Proposition</label>
              <input
                type="text"
                value={rootGoal}
                onChange={(e) => setRootGoal(e.target.value)}
                placeholder="A -> B -> A ∧ B"
                className="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-xs font-mono text-white focus:outline-none focus:border-blue-500"
              />
            </div>
          </div>

          {/* Search Action Buttons & Config Toggle */}
          <div className="flex flex-wrap items-center justify-between gap-3 pt-2">
            <div className="flex items-center gap-2">
              <button
                type="button"
                disabled={isSearching || !rootGoal}
                onClick={handleStartSearch}
                className="flex items-center gap-2 px-5 py-2.5 bg-gradient-to-r from-blue-600 to-indigo-600 hover:from-blue-500 hover:to-indigo-500 text-white text-xs font-mono font-bold rounded-lg shadow-lg shadow-blue-950/40 transition-all disabled:opacity-50"
              >
                <Play className={`w-4 h-4 ${isSearching ? 'animate-spin' : ''}`} />
                <span>{isSearching ? 'Solving Autonomous...' : 'Solve (Autonomous BFS)'}</span>
              </button>

              <button
                type="button"
                disabled={isSearching || !rootGoal}
                onClick={handleStepOnce}
                className="flex items-center gap-2 px-4 py-2.5 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-mono font-semibold rounded-lg border border-slate-700 transition-all disabled:opacity-50"
              >
                <StepForward className="w-4 h-4 text-blue-400" />
                <span>Step Once</span>
              </button>

              {isSearching && (
                <button
                  type="button"
                  onClick={handleStopSearch}
                  className="flex items-center gap-2 px-4 py-2.5 bg-rose-950/80 hover:bg-rose-900 border border-rose-700 text-rose-200 text-xs font-mono font-semibold rounded-lg transition-all"
                >
                  <Square className="w-4 h-4 text-rose-400" />
                  <span>Stop</span>
                </button>
              )}

              <button
                type="button"
                onClick={handleReset}
                className="flex items-center gap-1.5 px-3 py-2.5 bg-slate-900 hover:bg-slate-800 text-slate-400 hover:text-slate-200 text-xs font-mono rounded-lg border border-slate-800"
              >
                <RotateCcw className="w-3.5 h-3.5" />
                <span>Reset</span>
              </button>
            </div>

            <button
              type="button"
              onClick={() => setShowConfig(!showConfig)}
              className="flex items-center gap-1.5 text-xs font-mono text-slate-400 hover:text-slate-200 bg-slate-900 px-3 py-2 rounded-lg border border-slate-800"
            >
              <Sliders className="w-3.5 h-3.5 text-blue-400" />
              <span>{showConfig ? 'Hide Config' : 'Search Parameters'}</span>
            </button>
          </div>

          {/* Expandable Search Parameters */}
          {showConfig && (
            <div className="p-4 bg-slate-950 rounded-xl border border-slate-800 grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-4 pt-3">
              <div>
                <label className="block text-xs font-mono text-slate-400 mb-1">
                  Expansion Factor K: <span className="text-white font-bold">{config.expansionFactorK}</span>
                </label>
                <input
                  type="range"
                  min="1"
                  max="5"
                  value={config.expansionFactorK}
                  onChange={(e) => setConfig({ ...config, expansionFactorK: Number(e.target.value) })}
                  className="w-full h-1.5 bg-slate-800 rounded appearance-none cursor-pointer accent-blue-500"
                />
              </div>

              <div>
                <label className="block text-xs font-mono text-slate-400 mb-1">
                  Beam Width: <span className="text-white font-bold">{config.beamWidth}</span>
                </label>
                <input
                  type="range"
                  min="1"
                  max="8"
                  value={config.beamWidth}
                  onChange={(e) => setConfig({ ...config, beamWidth: Number(e.target.value) })}
                  className="w-full h-1.5 bg-slate-800 rounded appearance-none cursor-pointer accent-blue-500"
                />
              </div>

              <div>
                <label className="block text-xs font-mono text-slate-400 mb-1">
                  Max Depth: <span className="text-white font-bold">{config.maxDepth}</span>
                </label>
                <input
                  type="range"
                  min="2"
                  max="10"
                  value={config.maxDepth}
                  onChange={(e) => setConfig({ ...config, maxDepth: Number(e.target.value) })}
                  className="w-full h-1.5 bg-slate-800 rounded appearance-none cursor-pointer accent-blue-500"
                />
              </div>

              <div>
                <label className="block text-xs font-mono text-slate-400 mb-1">
                  Thinking Budget: <span className="text-white font-bold">{config.reasoningBudget} tok</span>
                </label>
                <input
                  type="range"
                  min="0"
                  max="512"
                  step="32"
                  value={config.reasoningBudget}
                  onChange={(e) => setConfig({ ...config, reasoningBudget: Number(e.target.value) })}
                  className="w-full h-1.5 bg-slate-800 rounded appearance-none cursor-pointer accent-purple-500"
                />
              </div>
            </div>
          )}
        </div>

        {/* Real-time Search Telemetry Counters */}
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
          <div className="bg-slate-950/70 p-4 rounded-xl border border-slate-800 flex items-center justify-between">
            <div>
              <div className="text-xs text-slate-400 font-mono">Explored Nodes</div>
              <div className="text-xl font-bold font-mono text-white mt-0.5">{nodesList.length}</div>
              <span className="text-[10px] text-slate-500 font-mono">({prunedCount} pruned)</span>
            </div>
            <Layers className="w-6 h-6 text-blue-400 opacity-60" />
          </div>

          <div className="bg-slate-950/70 p-4 rounded-xl border border-slate-800 flex items-center justify-between">
            <div>
              <div className="text-xs text-slate-400 font-mono">Open Queue</div>
              <div className="text-xl font-bold font-mono text-amber-400 mt-0.5">{openCount}</div>
              <span className="text-[10px] text-slate-500 font-mono">active beams</span>
            </div>
            <Search className="w-6 h-6 text-amber-400 opacity-60" />
          </div>

          <div className="bg-slate-950/70 p-4 rounded-xl border border-slate-800 flex items-center justify-between">
            <div>
              <div className="text-xs text-slate-400 font-mono">Proven Nodes</div>
              <div className="text-xl font-bold font-mono text-emerald-400 mt-0.5">{provenCount}</div>
              <span className="text-[10px] text-emerald-500 font-mono">{activeStepGoal ? 'Active step' : 'Ready'}</span>
            </div>
            <CheckCircle2 className="w-6 h-6 text-emerald-400 opacity-60" />
          </div>

          <div className="bg-slate-950/70 p-4 rounded-xl border border-slate-800 flex items-center justify-between">
            <div>
              <div className="text-xs text-slate-400 font-mono">Throughput</div>
              <div className="text-xl font-bold font-mono text-teal-300 mt-0.5">
                {tokSpeed.toFixed(1)} <span className="text-xs font-normal text-slate-400">tok/s</span>
              </div>
              <span className="text-[10px] text-teal-500 font-mono">{vramMB} MB VRAM</span>
            </div>
            <Activity className="w-6 h-6 text-teal-400 opacity-60" />
          </div>
        </div>

        {/* Live Proof Tree & Inspector Grid */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          {/* Left: Proof Tree Exploration View */}
          <div className="lg:col-span-7 bg-slate-950 p-5 rounded-xl border border-slate-800 flex flex-col h-[480px]">
            <div className="flex items-center justify-between border-b border-slate-800 pb-3 mb-4">
              <div className="flex items-center gap-2">
                <Workflow className="w-4 h-4 text-blue-400" />
                <h3 className="text-xs font-bold text-white font-mono uppercase tracking-wider">
                  Live Proof DAG / Search Tree
                </h3>
              </div>
              <span className="text-[11px] font-mono text-slate-400">
                {nodesList.length === 0 ? 'Queue empty' : `${nodesList.length} total nodes`}
              </span>
            </div>

            <div className="flex-1 overflow-y-auto space-y-2.5 pr-2">
              {nodesList.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-full text-slate-500 font-mono text-xs space-y-2">
                  <Workflow className="w-8 h-8 text-slate-700" />
                  <span>No search tree active. Click "Solve" to start autonomous BFS.</span>
                </div>
              ) : (
                nodesList.map((node) => (
                  <div
                    key={node.id}
                    onClick={() => setSelectedNodeId(node.id)}
                    className={`p-3 rounded-lg border font-mono text-xs cursor-pointer transition-all ${
                      selectedNodeId === node.id
                        ? 'bg-slate-900 border-blue-500 shadow-md'
                        : 'bg-slate-950/80 border-slate-800/80 hover:border-slate-700'
                    }`}
                    style={{ marginLeft: `${Math.min(node.depth * 18, 90)}px` }}
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <span
                          className={`w-2 h-2 rounded-full ${
                            node.status === 'proven'
                              ? 'bg-emerald-400 shadow-sm shadow-emerald-400'
                              : node.status === 'expanded'
                              ? 'bg-blue-400'
                              : node.status === 'open'
                              ? 'bg-amber-400'
                              : 'bg-slate-600'
                          }`}
                        />
                        <span className="font-bold text-white">{node.goal}</span>
                      </div>
                      <div className="flex items-center gap-2">
                        <span className="px-1.5 py-0.5 rounded text-[10px] bg-blue-950/80 border border-blue-800 text-blue-300">
                          {(node.genrmScore * 100).toFixed(0)}%
                        </span>
                        <span
                          className={`px-1.5 py-0.5 rounded text-[10px] uppercase font-semibold ${
                            node.status === 'proven'
                              ? 'bg-emerald-950 text-emerald-300 border border-emerald-800'
                              : node.status === 'expanded'
                              ? 'bg-blue-950 text-blue-300 border border-blue-800'
                              : 'bg-slate-800 text-slate-400'
                          }`}
                        >
                          {node.status}
                        </span>
                      </div>
                    </div>

                    {node.tacticApplied && (
                      <div className="mt-1.5 flex items-center gap-1.5 text-[11px] text-emerald-400">
                        <ChevronRight className="w-3 h-3 text-slate-500 shrink-0" />
                        <span className="font-semibold text-emerald-300">Tactic: {node.tacticApplied}</span>
                      </div>
                    )}
                  </div>
                ))
              )}
            </div>
          </div>

          {/* Right: Node Detail & Thinking Inspector */}
          <div className="lg:col-span-5 bg-slate-950 p-5 rounded-xl border border-slate-800 flex flex-col h-[480px]">
            <div className="flex items-center justify-between border-b border-slate-800 pb-3 mb-4">
              <div className="flex items-center gap-2">
                <Brain className="w-4 h-4 text-purple-400" />
                <h3 className="text-xs font-bold text-white font-mono uppercase tracking-wider">
                  Node & Thinking Inspector
                </h3>
              </div>
              {selectedNode && (
                <span className="text-[11px] font-mono text-purple-400">
                  Depth: {selectedNode.depth} | Q: {selectedNode.cumulativeScore.toFixed(3)}
                </span>
              )}
            </div>

            {selectedNode ? (
              <div className="flex-1 overflow-y-auto space-y-4 text-xs font-mono pr-2">
                <div>
                  <span className="text-slate-400 block text-[11px] font-semibold mb-1">Goal State:</span>
                  <div className="p-2.5 bg-slate-900 rounded-lg border border-slate-800 text-white font-bold">
                    {selectedNode.goal}
                  </div>
                </div>

                {selectedNode.tacticApplied && (
                  <div>
                    <span className="text-slate-400 block text-[11px] font-semibold mb-1">Applied Tactic AST:</span>
                    <div className="p-2.5 bg-slate-900 rounded-lg border border-emerald-900/60 text-emerald-300 font-bold">
                      {selectedNode.tacticApplied}
                    </div>
                  </div>
                )}

                {selectedNode.thinkingTrace && (
                  <div>
                    <span className="text-purple-400 block text-[11px] font-semibold mb-1 flex items-center gap-1">
                      <Brain className="w-3 h-3 text-purple-400" />
                      <span>Actor Scratchpad Reasoning (&lt;think&gt;):</span>
                    </span>
                    <div className="p-3 bg-purple-950/20 border border-purple-900/40 rounded-lg text-slate-300 leading-relaxed text-[11px]">
                      {selectedNode.thinkingTrace}
                    </div>
                  </div>
                )}

                <div className="grid grid-cols-2 gap-2 pt-1">
                  <div className="p-2.5 bg-slate-900 rounded-lg border border-slate-800">
                    <span className="text-[10px] text-slate-500 block">GenRM Confidence:</span>
                    <span className="text-sm font-bold text-blue-400">
                      {(selectedNode.genrmScore * 100).toFixed(1)}%
                    </span>
                  </div>
                  <div className="p-2.5 bg-slate-900 rounded-lg border border-slate-800">
                    <span className="text-[10px] text-slate-500 block">Status:</span>
                    <span
                      className={`text-sm font-bold uppercase ${
                        selectedNode.status === 'proven' ? 'text-emerald-400' : 'text-slate-300'
                      }`}
                    >
                      {selectedNode.status}
                    </span>
                  </div>
                </div>
              </div>
            ) : (
              <div className="flex flex-col items-center justify-center h-full text-slate-500 font-mono text-xs">
                <span>Select any node in the tree to inspect scratchpad reasoning and GenRM scores</span>
              </div>
            )}
          </div>
        </div>

        {/* Proven Solution Box */}
        {searchResult?.success && (
          <div className="bg-slate-950 p-5 rounded-xl border border-emerald-800/80 shadow-2xl space-y-3">
            <div className="flex items-center justify-between border-b border-emerald-900/60 pb-3">
              <div className="flex items-center gap-2">
                <CheckCircle2 className="w-5 h-5 text-emerald-400" />
                <h3 className="text-sm font-bold text-white font-mono">
                  Autonomous Proof Successfully Closed! ({searchResult.elapsedMs.toFixed(0)} ms)
                </h3>
              </div>
              <button
                type="button"
                onClick={() => copyProofScript(searchResult.tacticScript)}
                className="flex items-center gap-1.5 px-3 py-1.5 bg-emerald-950 border border-emerald-700 text-emerald-300 text-xs font-mono rounded-lg hover:bg-emerald-900 transition-all"
              >
                {copiedScript ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
                <span>{copiedScript ? 'Copied' : 'Copy Lean 4 Script'}</span>
              </button>
            </div>

            <pre className="p-4 bg-slate-900 rounded-lg border border-slate-800 text-emerald-300 font-mono text-xs overflow-x-auto leading-relaxed">
              {searchResult.tacticScript}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
};
