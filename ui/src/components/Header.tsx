import React, { useState, useEffect } from 'react';
import {
  Cpu,
  GitCommit,
  Brain,
  Sparkles,
  Activity,
  HardDrive,
  CloudDownload,
  CheckCircle2,
  AlertTriangle,
  HelpCircle,
  Trophy,
} from 'lucide-react';
import { gemmaEdgeController, InitProgressReport, LlmEngineState } from '../services/llmController';
import { meshClient, MeshTelemetry } from '../services/meshClient';

export type ActiveTabId = 'contribute' | 'volunteer' | 'dag' | 'playground' | 'gemma4' | 'telemetry';

export interface HeaderProps {
  activeTab: ActiveTabId;
  setActiveTab: (tab: ActiveTabId) => void;
  onOpenTour?: () => void;
  blocksCount?: number;
}

export const Header: React.FC<HeaderProps> = ({
  activeTab,
  setActiveTab,
  onOpenTour,
  blocksCount = 0,
}) => {
  const [engineState, setEngineState] = useState<LlmEngineState>(gemmaEdgeController.state);
  const [progressReport, setProgressReport] = useState<InitProgressReport>(gemmaEdgeController.currentProgress);
  const [meshTelemetry, setMeshTelemetry] = useState<MeshTelemetry>(meshClient.getTelemetry());
  const [globalStats, setGlobalStats] = useState<{
    active_workers: number;
    proven_nodes: number;
    total_nodes: number;
    tasks_in_queue: number;
  }>({
    active_workers: 1,
    proven_nodes: 0,
    total_nodes: 0,
    tasks_in_queue: 0,
  });

  useEffect(() => {
    const unsubProgress = gemmaEdgeController.onInitProgress((report) => {
      setProgressReport(report);
    });

    const unsubState = gemmaEdgeController.onStateChange((st) => {
      setEngineState(st);
    });

    const unsubMesh = meshClient.on('*', () => {
      setMeshTelemetry(meshClient.getTelemetry());
    });

    meshClient
      .getCoordinatorTelemetry()
      .then((stats) => {
        if (stats) setGlobalStats(stats);
      })
      .catch(() => {});

    const pollStats = setInterval(() => {
      if (meshClient.getTelemetry().status === 'connected') {
        meshClient
          .getCoordinatorTelemetry()
          .then((stats) => {
            if (stats) setGlobalStats(stats);
          })
          .catch(() => {});
      }
    }, 2500);

    return () => {
      unsubProgress();
      unsubState();
      unsubMesh();
      clearInterval(pollStats);
    };
  }, []);

  const progressPercent = Math.min(100, Math.max(0, Math.round(progressReport.progress * 100)));
  const isCacheSource = progressReport.text.toLowerCase().includes('cache');
  const isNetworkSource =
    progressReport.text.toLowerCase().includes('download') ||
    progressReport.text.toLowerCase().includes('fetch') ||
    progressReport.text.toLowerCase().includes('network');

  const vramGB = (gemmaEdgeController.vramAllocated / 1024).toFixed(2);
  const displayVramText =
    gemmaEdgeController.vramAllocated > 0
      ? `Gemma 4 Edge (${vramGB} GB VRAM allocated)`
      : 'Gemma 4 Edge (1.85 GB VRAM allocated)';

  const activeWorkerCount = meshTelemetry.networkWorkers || globalStats.active_workers || 1;
  const provenCount = globalStats.proven_nodes || blocksCount || 0;
  const totalTargetCount = globalStats.total_nodes || Math.max(provenCount, 12);

  const isContributeActive = activeTab === 'contribute' || activeTab === 'volunteer';
  const isDagActive = activeTab === 'dag';
  const isPlaygroundActive = activeTab === 'playground' || activeTab === 'gemma4';
  const isTelemetryActive = activeTab === 'telemetry';

  return (
    <header className="flex flex-col bg-slate-900 border-b border-slate-800 z-10">
      <div className="flex flex-wrap items-center justify-between px-6 py-3 gap-4">
        {/* Brand & Live Reactive Swarm Telemetry */}
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2.5">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-tr from-blue-600 via-indigo-600 to-purple-600 flex items-center justify-center font-serif text-lg font-bold text-white shadow-md shadow-blue-500/20">
              ℬ
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h1 className="text-sm font-black tracking-wider text-white">BourbakiMesh</h1>
                <span className="px-1.5 py-0.2 text-[9px] font-mono font-bold bg-indigo-950/90 text-indigo-300 border border-indigo-700/50 rounded">
                  Phase 5
                </span>
              </div>
              <div className="text-[10px] text-slate-400 font-mono">Distributed Formal Reasoning Swarm</div>
            </div>
          </div>

          {/* Live Reactive Swarm Badges (Replaced Obsolete Static Badges) */}
          <div className="hidden lg:flex items-center gap-2.5 pl-4 border-l border-slate-800 text-xs font-mono">
            {/* Worker Swarm Badge */}
            <div className="flex items-center gap-1.5 px-2.5 py-1 bg-slate-950/80 border border-slate-800/90 rounded-lg shadow-sm">
              {meshTelemetry.status === 'connected' ? (
                <>
                  <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
                  <span className="font-bold text-emerald-400">{activeWorkerCount}</span>
                  <span className="text-slate-300">
                    Active {activeWorkerCount === 1 ? 'Worker' : 'Workers'}
                  </span>
                </>
              ) : meshTelemetry.status === 'connecting' || meshTelemetry.status === 'reconnecting' ? (
                <>
                  <span className="w-2 h-2 rounded-full bg-amber-400 animate-ping" />
                  <span className="text-amber-300 font-semibold">Connecting...</span>
                </>
              ) : (
                <>
                  <span className="w-2 h-2 rounded-full bg-slate-500" />
                  <span className="text-slate-400">Disconnected (0 Nodes)</span>
                </>
              )}
            </div>

            {/* Proof Progress Badge */}
            <div className="flex items-center gap-1.5 px-2.5 py-1 bg-slate-950/80 border border-slate-800/90 rounded-lg shadow-sm">
              <Trophy className="w-3.5 h-3.5 text-amber-400" />
              <span className="font-bold text-amber-300">{provenCount}</span>
              <span className="text-slate-500">/</span>
              <span className="text-slate-400">{totalTargetCount} Proven</span>
            </div>

            {/* Active Edge Model Pill */}
            <div className="flex items-center gap-2 px-2.5 py-1 bg-slate-950/80 border border-slate-800/90 rounded-lg shadow-sm">
              <Brain className="w-3.5 h-3.5 text-teal-400" />
              <span className="text-slate-200 font-semibold">Gemma 4 Edge (2B-IT W4A16)</span>
              <span className="px-1.5 py-0.5 text-[9px] bg-teal-950 border border-teal-700/50 text-teal-300 rounded font-mono font-semibold">
                WebGPU (shader-f16)
              </span>
            </div>
          </div>
        </div>

        {/* Real-time Gemma 4 Model Loading & VRAM Status Indicator */}
        <div className="flex items-center gap-3 min-w-[280px] max-w-[420px] flex-1 justify-center md:justify-end">
          {engineState === 'loading' && (
            <div className="w-full bg-slate-950/90 border border-teal-500/30 rounded-xl p-2 px-3 shadow-lg shadow-teal-950/30">
              <div className="flex items-center justify-between gap-2 mb-1.5">
                <div className="flex items-center gap-1.5">
                  <span className="w-2 h-2 rounded-full bg-teal-400 animate-ping" />
                  <span className="text-[11px] font-mono font-bold text-teal-300">
                    Loading Gemma 4 ({progressPercent}%)
                  </span>
                </div>
                {/* Source Indicator Pill */}
                {isCacheSource ? (
                  <span className="flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded bg-cyan-950/90 border border-cyan-600/50 text-cyan-300 font-mono">
                    <HardDrive className="w-2.5 h-2.5 text-cyan-400" />
                    Browser Cache
                  </span>
                ) : isNetworkSource ? (
                  <span className="flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded bg-indigo-950/90 border border-indigo-600/50 text-indigo-300 font-mono">
                    <CloudDownload className="w-2.5 h-2.5 text-indigo-400" />
                    Network Download
                  </span>
                ) : (
                  <span className="flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded bg-purple-950/90 border border-purple-600/50 text-purple-300 font-mono">
                    <Cpu className="w-2.5 h-2.5 text-purple-400" />
                    WebGPU Shaders
                  </span>
                )}
              </div>

              {/* Progress Bar Track */}
              <div className="w-full h-1.5 bg-slate-900 rounded-full border border-slate-800 overflow-hidden">
                <div
                  className="h-full bg-gradient-to-r from-teal-500 via-emerald-400 to-cyan-400 transition-all duration-300 ease-out shadow-sm shadow-teal-400/50"
                  style={{ width: `${Math.max(5, progressPercent)}%` }}
                />
              </div>

              <div className="flex items-center justify-between text-[10px] text-slate-400 font-mono mt-1">
                <span className="truncate max-w-[260px]">{progressReport.text}</span>
                {progressReport.timeElapsed > 0 && (
                  <span className="text-slate-500">{progressReport.timeElapsed.toFixed(1)}s</span>
                )}
              </div>
            </div>
          )}

          {engineState === 'ready' && (
            <div className="flex items-center gap-2 px-3 py-1.5 bg-emerald-950/80 border border-emerald-500/40 text-emerald-300 rounded-lg text-xs font-mono shadow-md shadow-emerald-950/40">
              <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
              <Cpu className="w-3.5 h-3.5 text-emerald-400" />
              <span className="font-bold">{displayVramText}</span>
              <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400 ml-0.5" />
            </div>
          )}

          {engineState === 'idle' && (
            <div className="flex items-center gap-2 px-3 py-1.5 bg-slate-800/60 border border-slate-700/50 text-slate-400 rounded-lg text-xs font-mono">
              <Brain className="w-3.5 h-3.5 text-slate-500" />
              <span>Gemma 4 Edge (Standby)</span>
            </div>
          )}

          {engineState === 'error' && (
            <div className="flex items-center gap-2 px-3 py-1.5 bg-rose-950/80 border border-rose-500/40 text-rose-300 rounded-lg text-xs font-mono">
              <AlertTriangle className="w-3.5 h-3.5 text-rose-400" />
              <span>Gemma 4 Edge (Init Error)</span>
            </div>
          )}
        </div>

        {/* 4 Pillars Navigation & Quick Tour Button */}
        <div className="flex items-center gap-2">
          <nav className="flex items-center gap-1 bg-slate-950 p-1 rounded-xl border border-slate-800 text-xs">
            {/* Pillar 1: Contribute Cycles */}
            <button
              type="button"
              onClick={() => setActiveTab('contribute')}
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg font-semibold transition-colors ${
                isContributeActive
                  ? 'bg-gradient-to-r from-blue-600 to-indigo-600 text-white shadow shadow-blue-500/20'
                  : 'text-slate-400 hover:text-slate-200'
              }`}
            >
              <Sparkles className="w-3.5 h-3.5 text-blue-400" />
              Contribute Cycles
            </button>

            {/* Pillar 2: Proof DAG Explorer */}
            <button
              type="button"
              onClick={() => setActiveTab('dag')}
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg font-semibold transition-colors ${
                isDagActive
                  ? 'bg-blue-600 text-white shadow'
                  : 'text-slate-400 hover:text-slate-200'
              }`}
            >
              <GitCommit className="w-3.5 h-3.5" />
              Proof DAG Explorer
            </button>

            {/* Pillar 3: Model Playground */}
            <button
              type="button"
              onClick={() => setActiveTab('playground')}
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg font-semibold transition-colors ${
                isPlaygroundActive
                  ? 'bg-gradient-to-r from-emerald-600 to-teal-600 text-white shadow shadow-emerald-500/20'
                  : 'text-slate-400 hover:text-slate-200'
              }`}
            >
              <Brain className="w-3.5 h-3.5 text-emerald-400" />
              Model Playground
            </button>

            {/* Pillar 4: Flight Telemetry */}
            <button
              type="button"
              onClick={() => setActiveTab('telemetry')}
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg font-semibold transition-colors ${
                isTelemetryActive
                  ? 'bg-blue-600 text-white shadow'
                  : 'text-slate-400 hover:text-slate-200'
              }`}
            >
              <Activity className="w-3.5 h-3.5" />
              Flight Telemetry
            </button>
          </nav>

          {/* Quick Tour / Help Button */}
          {onOpenTour && (
            <button
              type="button"
              onClick={onOpenTour}
              className="flex items-center gap-1.5 px-3 py-2 bg-indigo-950/80 hover:bg-indigo-900 border border-indigo-700/60 text-indigo-200 hover:text-white rounded-xl text-xs font-mono font-semibold transition-all shadow-md shadow-indigo-950/40"
              title="Open Interactive Guided Tour"
            >
              <HelpCircle className="w-4 h-4 text-indigo-400" />
              <span className="hidden sm:inline">Quick Tour</span>
            </button>
          )}
        </div>
      </div>
    </header>
  );
};
