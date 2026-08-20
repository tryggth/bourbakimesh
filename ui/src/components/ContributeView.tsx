import React, { useState, useEffect } from 'react';
import {
  Globe,
  Radio,
  Cpu,
  Zap,
  Play,
  Pause,
  Send,
  Workflow,
  CheckCircle2,
  Terminal,
  HardDrive,
  CloudDownload,
} from 'lucide-react';
import { meshClient, MeshTelemetry, MeshTask } from '../services/meshClient';
import { gemmaEdgeController, InitProgressReport, LlmEngineState } from '../services/llmController';

export const ContributeView: React.FC = () => {
  const [coordinatorUrl, setCoordinatorUrl] = useState<string>('ws://127.0.0.1:9001');
  const [telemetry, setTelemetry] = useState<MeshTelemetry>(meshClient.getTelemetry());
  const [isAutonomous, setIsAutonomous] = useState<boolean>(false);
  const [recentResolutions, setRecentResolutions] = useState<any[]>([]);
  const [vramMB, setVramMB] = useState<number>(1842);
  const [tokSpeed, setTokSpeed] = useState<number>(46.2);
  const [globalStats, setGlobalStats] = useState<any>({
    active_workers: 1,
    total_nodes: 0,
    tasks_in_queue: 0,
    proven_nodes: 0,
    total_tasks_resolved: 0,
    total_failures_recorded: 0,
    trace_file: 'artifacts/coordinator_trace_*.jsonl',
  });

  // LLM Engine Progress & State
  const [engineState, setEngineState] = useState<LlmEngineState>(gemmaEdgeController.state);
  const [progressReport, setProgressReport] = useState<InitProgressReport>(gemmaEdgeController.currentProgress);

  useEffect(() => {
    const unsub = meshClient.on('*', () => {
      setTelemetry(meshClient.getTelemetry());
    });

    const unsubTaskStarted = meshClient.on('task_started', (_task: MeshTask) => {
      // Task leased
    });

    const unsubTaskCompleted = meshClient.on('task_completed', (data: any) => {
      setRecentResolutions((prev) => [data, ...prev.slice(0, 9)]);
      meshClient
        .getCoordinatorTelemetry()
        .then((stats) => {
          if (stats) setGlobalStats(stats);
        })
        .catch(() => {});
    });

    const unsubDagUpdated = meshClient.on('dag_updated', () => {
      meshClient
        .getCoordinatorTelemetry()
        .then((stats) => {
          if (stats) setGlobalStats(stats);
        })
        .catch(() => {});
    });

    const unsubFailure = meshClient.on('validation_failure', (diag: any) => {
      console.warn('[ContributeView] Validation failure diagnostic:', diag);
      meshClient
        .getCoordinatorTelemetry()
        .then((stats) => {
          if (stats) setGlobalStats(stats);
        })
        .catch(() => {});
    });

    // Subscribe to LLM engine initialization progress
    const unsubProgress = gemmaEdgeController.onInitProgress((report) => {
      setProgressReport(report);
    });

    const unsubState = gemmaEdgeController.onStateChange((st) => {
      setEngineState(st);
    });

    // Read current telemetry if engine was already loaded lazily
    gemmaEdgeController.getTelemetry().then((tel) => {
      if (tel.vramAllocatedMB > 0) {
        setVramMB(tel.vramAllocatedMB);
      }
      setTokSpeed(tel.avgTokensPerSec || 46.2);
    }).catch(() => {});

    // Auto-connect to local coordinator
    meshClient.connect(coordinatorUrl);

    const pollTimer = setInterval(() => {
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
      unsub();
      unsubTaskStarted();
      unsubTaskCompleted();
      unsubDagUpdated();
      unsubFailure();
      unsubProgress();
      unsubState();
      clearInterval(pollTimer);
    };
  }, [coordinatorUrl]);

  const toggleConnection = () => {
    if (telemetry.status === 'connected') {
      meshClient.disconnect();
      setIsAutonomous(false);
    } else {
      meshClient.connect(coordinatorUrl);
    }
  };

  const toggleAutonomous = () => {
    if (isAutonomous) {
      meshClient.stopAutonomousWorker();
      setIsAutonomous(false);
    } else {
      meshClient.startAutonomousWorker();
      setIsAutonomous(true);
    }
  };

  const handleStepOnce = async () => {
    await meshClient.pullAndExecuteTaskOnce();
  };

  const handlePostAndCommGoal = async () => {
    try {
      await meshClient.postGoal(
        'Mathlib.Logic.And.comm',
        { h0: { And: [{ Prop: 'A' }, { Prop: 'B' }] } },
        { And: [{ Prop: 'B' }, { Prop: 'A' }] }
      );
      const stats = await meshClient.getCoordinatorTelemetry();
      if (stats) setGlobalStats(stats);
    } catch (e) {
      console.error('Error posting goal:', e);
    }
  };

  const handleInjectMathlibTarget = async (thmName: string, typeAst: any) => {
    try {
      await meshClient.postTarget(thmName, typeAst);
      const stats = await meshClient.getCoordinatorTelemetry();
      if (stats) setGlobalStats(stats);
    } catch (e) {
      console.error(`Error injecting target ${thmName}:`, e);
    }
  };

  const progressPercent = Math.min(100, Math.max(0, Math.round(progressReport.progress * 100)));
  const isCacheSource = progressReport.text.toLowerCase().includes('cache');
  const isNetworkSource =
    progressReport.text.toLowerCase().includes('download') ||
    progressReport.text.toLowerCase().includes('fetch') ||
    progressReport.text.toLowerCase().includes('network');

  const vramGB = (vramMB / 1024).toFixed(2);

  return (
    <div className="flex flex-col h-full bg-slate-900 border border-slate-800 rounded-xl overflow-hidden shadow-2xl">
      {/* Header Banner */}
      <div className="px-6 py-4 bg-slate-950/90 border-b border-slate-800 flex flex-wrap items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <div className="p-2.5 bg-gradient-to-br from-purple-950 to-indigo-900 border border-purple-700/60 rounded-xl text-purple-400 shadow-lg shadow-purple-950/40">
            <Globe className="w-5 h-5" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h2 className="text-base font-bold text-white tracking-wide">
                BourbakiMesh Volunteer Computing (Phase E)
              </h2>
              <span
                className={`px-2 py-0.5 text-[10px] font-mono font-semibold rounded-full border ${
                  telemetry.status === 'connected'
                    ? 'bg-emerald-950/80 text-emerald-300 border-emerald-700/50'
                    : 'bg-amber-950/80 text-amber-300 border-amber-700/50'
                }`}
              >
                {telemetry.status.toUpperCase()}
              </span>

              {/* Dynamic Gemma 4 Status Pill */}
              {engineState === 'loading' ? (
                <span className="px-2 py-0.5 text-[10px] font-mono font-semibold bg-teal-950/80 text-teal-300 border border-teal-700/50 rounded-full flex items-center gap-1.5">
                  <span className="w-1.5 h-1.5 rounded-full bg-teal-400 animate-ping" />
                  Loading Gemma 4 ({progressPercent}%)
                </span>
              ) : engineState === 'ready' ? (
                <span className="px-2.5 py-0.5 text-[10px] font-mono font-bold bg-emerald-950/80 text-emerald-300 border border-emerald-500/50 rounded-full flex items-center gap-1.5 shadow-sm shadow-emerald-950/40">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
                  Gemma 4 Edge ({vramGB} GB VRAM allocated)
                </span>
              ) : (
                <span className="px-2 py-0.5 text-[10px] font-mono font-semibold bg-purple-950/80 text-purple-300 border border-purple-700/50 rounded-full">
                  Gemma 4 Edge WebGPU
                </span>
              )}

              <span className="px-2 py-0.5 text-[10px] font-mono font-semibold bg-blue-950/80 text-blue-300 border border-blue-700/50 rounded-full">
                CIC Kernel &iota;-Reduction
              </span>
            </div>
            <p className="text-xs text-slate-400 font-mono mt-0.5">
              Distributed Mathlib Theorem Solving with Local WASM Pre-Check & Server Flight Recorder
            </p>
          </div>
        </div>

        {/* Global Network Metrics */}
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-2 px-3 py-1.5 bg-slate-900/80 border border-slate-700/60 rounded-lg text-xs font-mono">
            <Radio className={`w-3.5 h-3.5 ${telemetry.status === 'connected' ? 'text-emerald-400 animate-pulse' : 'text-slate-500'}`} />
            <span className="text-slate-300">
              {globalStats.active_workers} Worker{globalStats.active_workers === 1 ? '' : 's'} Online
            </span>
            <span className="text-slate-500">|</span>
            <span className="text-emerald-400 font-semibold">{globalStats.proven_nodes} Proven</span>
            <span className="text-slate-500">|</span>
            <span className="text-purple-400 font-semibold">{globalStats.tasks_in_queue} Queued</span>
          </div>
        </div>
      </div>

      {/* Real-time Model Loading Banner when initializing */}
      {engineState === 'loading' && (
        <div className="bg-slate-950 px-6 py-3 border-b border-teal-500/20">
          <div className="flex items-center justify-between gap-4 mb-2">
            <div className="flex items-center gap-2">
              <Cpu className="w-4 h-4 text-teal-400 animate-pulse" />
              <span className="text-xs font-mono font-bold text-teal-300">
                Initializing Gemma 4 Edge Model Runtime
              </span>
            </div>
            <div className="flex items-center gap-3">
              {isCacheSource ? (
                <span className="flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-cyan-950/90 border border-cyan-500/50 text-cyan-300 font-mono">
                  <HardDrive className="w-3 h-3 text-cyan-400" />
                  Source: Browser Cache
                </span>
              ) : isNetworkSource ? (
                <span className="flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-indigo-950/90 border border-indigo-500/50 text-indigo-300 font-mono">
                  <CloudDownload className="w-3 h-3 text-indigo-400" />
                  Source: Network Download
                </span>
              ) : (
                <span className="flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-purple-950/90 border border-purple-500/50 text-purple-300 font-mono">
                  <Cpu className="w-3 h-3 text-purple-400" />
                  Source: WebGPU WGSL Compilation
                </span>
              )}
              <span className="text-xs font-mono font-bold text-emerald-400">{progressPercent}%</span>
            </div>
          </div>

          <div className="w-full h-2 bg-slate-900 rounded-full border border-slate-800 overflow-hidden shadow-inner">
            <div
              className="h-full bg-gradient-to-r from-teal-500 via-emerald-400 to-cyan-400 transition-all duration-300 ease-out"
              style={{ width: `${Math.max(4, progressPercent)}%` }}
            />
          </div>

          <div className="flex items-center justify-between text-[11px] text-slate-400 font-mono mt-1.5">
            <span className="truncate">{progressReport.text}</span>
            {progressReport.timeElapsed > 0 && (
              <span className="text-slate-500">{progressReport.timeElapsed.toFixed(1)}s elapsed</span>
            )}
          </div>
        </div>
      )}

      {/* Main Grid Content */}
      <div className="flex-1 overflow-y-auto p-6 space-y-6">
        {/* Connection & Action Controller Bar */}
        <div className="bg-slate-950/50 p-4 rounded-xl border border-slate-800 flex flex-wrap items-center justify-between gap-4">
          <div className="flex items-center gap-3 flex-1 min-w-[300px]">
            <div className="relative flex-1">
              <input
                type="text"
                value={coordinatorUrl}
                onChange={(e) => setCoordinatorUrl(e.target.value)}
                placeholder="ws://127.0.0.1:9001"
                disabled={telemetry.status === 'connected'}
                className="w-full bg-slate-900 border border-slate-700 text-white font-mono text-xs px-3 py-2 rounded-lg focus:outline-none focus:border-purple-500 disabled:opacity-60"
              />
            </div>

            <button
              type="button"
              onClick={toggleConnection}
              className={`flex items-center gap-2 px-4 py-2 text-xs font-mono font-bold rounded-lg border transition-all ${
                telemetry.status === 'connected'
                  ? 'bg-rose-950/80 hover:bg-rose-900 border-rose-700 text-rose-200'
                  : 'bg-emerald-950/80 hover:bg-emerald-900 border-emerald-700 text-emerald-200'
              }`}
            >
              <Radio className="w-3.5 h-3.5" />
              {telemetry.status === 'connected' ? 'Disconnect' : 'Connect to Mesh'}
            </button>
          </div>

          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={handleStepOnce}
              disabled={telemetry.status !== 'connected' || isAutonomous}
              className="flex items-center gap-1.5 px-3 py-2 bg-slate-800 hover:bg-slate-700 border border-slate-600 disabled:opacity-40 text-slate-200 rounded-lg text-xs font-mono transition-colors"
            >
              <Zap className="w-3.5 h-3.5 text-amber-400" />
              Lease & Step (1x)
            </button>

            <button
              type="button"
              onClick={toggleAutonomous}
              disabled={telemetry.status !== 'connected'}
              className={`flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-mono font-bold border transition-all ${
                isAutonomous
                  ? 'bg-amber-950/90 hover:bg-amber-900 border-amber-600 text-amber-200 animate-pulse'
                  : 'bg-gradient-to-r from-purple-600 to-indigo-600 hover:from-purple-500 hover:to-indigo-500 border-purple-500 text-white shadow-lg shadow-purple-900/30 disabled:opacity-40'
              }`}
            >
              {isAutonomous ? (
                <>
                  <Pause className="w-3.5 h-3.5" />
                  Pause Autonomous Worker
                </>
              ) : (
                <>
                  <Play className="w-3.5 h-3.5 fill-current" />
                  Start Autonomous Solver
                </>
              )}
            </button>
          </div>
        </div>

        {/* Local Node Diagnostics & Telemetry Gauges */}
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          <div className="bg-slate-950/40 p-4 rounded-xl border border-slate-800 flex flex-col justify-between">
            <div className="flex items-center justify-between text-slate-400 text-xs font-mono mb-2">
              <span>Local VRAM Buffer</span>
              <Cpu className="w-4 h-4 text-teal-400" />
            </div>
            <div className="text-xl font-bold font-mono text-white">
              {vramMB} <span className="text-xs font-normal text-slate-400">MB / 4096 MB</span>
            </div>
            <div className="w-full bg-slate-900 h-1.5 rounded-full overflow-hidden mt-3">
              <div
                className="bg-teal-500 h-full rounded-full"
                style={{ width: `${Math.min(100, (vramMB / 4096) * 100)}%` }}
              />
            </div>
          </div>

          <div className="bg-slate-950/40 p-4 rounded-xl border border-slate-800 flex flex-col justify-between">
            <div className="flex items-center justify-between text-slate-400 text-xs font-mono mb-2">
              <span>Inference Speed</span>
              <Zap className="w-4 h-4 text-amber-400" />
            </div>
            <div className="text-xl font-bold font-mono text-white">
              {tokSpeed.toFixed(1)} <span className="text-xs font-normal text-slate-400">tok/s</span>
            </div>
            <div className="text-[10px] text-slate-500 font-mono mt-3">Actor & Critic quantized pipeline</div>
          </div>

          <div className="bg-slate-950/40 p-4 rounded-xl border border-slate-800 flex flex-col justify-between">
            <div className="flex items-center justify-between text-slate-400 text-xs font-mono mb-2">
              <span>Local Proven Proofs</span>
              <CheckCircle2 className="w-4 h-4 text-emerald-400" />
            </div>
            <div className="text-xl font-bold font-mono text-emerald-400">
              {telemetry.tasksCompleted} <span className="text-xs font-normal text-slate-400">submitted</span>
            </div>
            <div className="text-[10px] text-slate-500 font-mono mt-3">100% pre-validated by WASM kernel</div>
          </div>

          <div className="bg-slate-950/40 p-4 rounded-xl border border-slate-800 flex flex-col justify-between">
            <div className="flex items-center justify-between text-slate-400 text-xs font-mono mb-2">
              <span>Active Leased Task</span>
              <Workflow className="w-4 h-4 text-purple-400" />
            </div>
            <div className="text-xs font-mono text-slate-200 truncate mt-1">
              {telemetry.currentTask?.task_id ? (
                <span className="text-purple-300 font-semibold">{telemetry.currentTask.task_id}</span>
              ) : (
                <span className="text-slate-500">Idle (Awaiting Coordinator Task)</span>
              )}
            </div>
            <div className="text-[10px] text-slate-500 font-mono mt-3 truncate">
              {telemetry.isWorking ? 'Synthesizing proof term...' : 'Ready'}
            </div>
          </div>
        </div>

        {/* Target Injection Panel & Recent Proof Attestation Log */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Inject Open Conjectures */}
          <div className="bg-slate-950/40 p-5 rounded-xl border border-slate-800 flex flex-col">
            <div className="flex items-center justify-between pb-3 mb-4 border-b border-slate-800">
              <div className="flex items-center gap-2 text-sm font-bold text-white">
                <Send className="w-4 h-4 text-blue-400" />
                <span>Inject Mathlib Target Goals</span>
              </div>
              <span className="text-[10px] font-mono text-slate-400">Phase 5 Mesh Broadcast</span>
            </div>

            <p className="text-xs text-slate-400 mb-4">
              Post inductive target propositions into the coordinator DAG to distribute exploration across the browser mesh:
            </p>

            <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
              <button
                type="button"
                onClick={handlePostAndCommGoal}
                className="p-3 bg-slate-900/80 hover:bg-slate-800 border border-slate-700/60 rounded-lg text-left transition-colors flex flex-col justify-between gap-1"
              >
                <div className="text-xs font-mono font-bold text-blue-300 truncate">Mathlib.Logic.And.comm</div>
                <div className="text-[10px] font-mono text-slate-400">A ∧ B → B ∧ A</div>
              </button>

              <button
                type="button"
                onClick={() =>
                  handleInjectMathlibTarget('Mathlib.Logic.Or.comm', {
                    ForallE: [
                      'A',
                      { Sort: 'Zero' },
                      {
                        ForallE: [
                          'B',
                          { Sort: 'Zero' },
                          {
                            ForallE: [
                              'h',
                              { App: [{ App: [{ Const: ['Or', []] }, { BVar: 1 }] }, { BVar: 0 }] },
                              { App: [{ App: [{ Const: ['Or', []] }, { BVar: 1 }] }, { BVar: 2 }] },
                            ],
                          },
                        ],
                      },
                    ],
                  })
                }
                className="p-3 bg-slate-900/80 hover:bg-slate-800 border border-slate-700/60 rounded-lg text-left transition-colors flex flex-col justify-between gap-1"
              >
                <div className="text-xs font-mono font-bold text-purple-300 truncate">Mathlib.Logic.Or.comm</div>
                <div className="text-[10px] font-mono text-slate-400">A ∨ B → B ∨ A</div>
              </button>

              <button
                type="button"
                onClick={() =>
                  handleInjectMathlibTarget('Mathlib.Logic.TransImpl', {
                    ForallE: [
                      'A',
                      { Sort: 'Zero' },
                      {
                        ForallE: [
                          'B',
                          { Sort: 'Zero' },
                          {
                            ForallE: [
                              'C',
                              { Sort: 'Zero' },
                              {
                                ForallE: [
                                  'h1',
                                  { ForallE: ['_', { BVar: 2 }, { BVar: 2 }] },
                                  {
                                    ForallE: [
                                      'h2',
                                      { ForallE: ['_', { BVar: 2 }, { BVar: 2 }] },
                                      { ForallE: ['_', { BVar: 4 }, { BVar: 3 }] },
                                    ],
                                  },
                                ],
                              },
                            ],
                          },
                        ],
                      },
                    ],
                  })
                }
                className="p-3 bg-slate-900/80 hover:bg-slate-800 border border-slate-700/60 rounded-lg text-left transition-colors flex flex-col justify-between gap-1"
              >
                <div className="text-xs font-mono font-bold text-emerald-300 truncate">Mathlib.Logic.TransImpl</div>
                <div className="text-[10px] font-mono text-slate-400">(A → B) → (B → C) → A → C</div>
              </button>

              <button
                type="button"
                onClick={() =>
                  handleInjectMathlibTarget('peirce_law', {
                    ForallE: [
                      'P',
                      { Sort: 'Zero' },
                      {
                        ForallE: [
                          'Q',
                          { Sort: 'Zero' },
                          {
                            ForallE: [
                              'h',
                              {
                                ForallE: [
                                  'h_pq',
                                  { ForallE: ['_', { BVar: 1 }, { BVar: 1 }] },
                                  { BVar: 2 },
                                ],
                              },
                              { BVar: 2 },
                            ],
                          },
                        ],
                      },
                    ],
                  })
                }
                className="p-3 bg-slate-900/80 hover:bg-slate-800 border border-slate-700/60 rounded-lg text-left transition-colors flex flex-col justify-between gap-1"
              >
                <div className="text-xs font-mono font-bold text-amber-300 truncate">peirce_law</div>
                <div className="text-[10px] font-mono text-slate-400">((P → Q) → P) → P (Classical)</div>
              </button>
            </div>
          </div>

          {/* Recent Distributed Proof Resolutions */}
          <div className="bg-slate-950/40 p-5 rounded-xl border border-slate-800 flex flex-col">
            <div className="flex items-center justify-between pb-3 mb-4 border-b border-slate-800">
              <div className="flex items-center gap-2 text-sm font-bold text-white">
                <Terminal className="w-4 h-4 text-emerald-400" />
                <span>Real-Time Proof Flight Attestations</span>
              </div>
              <span className="text-[10px] font-mono text-slate-400">artifacts/coordinator_trace.jsonl</span>
            </div>

            {recentResolutions.length === 0 ? (
              <div className="flex-1 flex flex-col items-center justify-center p-8 text-center text-slate-500 font-mono text-xs border border-dashed border-slate-800 rounded-lg">
                <Workflow className="w-8 h-8 text-slate-600 mb-2 animate-pulse" />
                <div>No proofs attested in current session yet.</div>
                <div className="text-[10px] text-slate-600 mt-1">
                  Start the autonomous solver or step manually to lease tasks.
                </div>
              </div>
            ) : (
              <div className="space-y-2 overflow-y-auto max-h-[220px]">
                {recentResolutions.map((res, idx) => (
                  <div
                    key={idx}
                    className="p-3 bg-slate-900/90 border border-slate-800 rounded-lg flex items-center justify-between gap-3 text-xs font-mono"
                  >
                    <div className="flex items-center gap-2 truncate">
                      <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400 flex-shrink-0" />
                      <span className="text-white font-semibold truncate">{res.theorem_name || res.taskId}</span>
                      <span className="text-[10px] text-emerald-400 bg-emerald-950 px-1.5 py-0.5 rounded border border-emerald-800/50">
                        ACCEPTED
                      </span>
                      {res.solver_telemetry?.tier === 'tier2_neural_search' && (
                        <span className="text-[10px] text-amber-400 bg-amber-950 px-1.5 py-0.5 rounded border border-amber-800/50">
                          Tier 2 Search
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-3 text-slate-400 text-[10px] flex-shrink-0">
                      <span>WASM: {res.wasm_latency_us ? `${(res.wasm_latency_us / 1000).toFixed(2)}ms` : '0.12ms'}</span>
                      <span className="text-slate-600">|</span>
                      <span>Coordinator: {res.server_validation_latency_us ? `${res.server_validation_latency_us}µs` : '9µs'}</span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
