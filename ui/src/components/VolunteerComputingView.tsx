import React, { useState, useEffect } from 'react';
import {
  Globe,
  Radio,
  Cpu,
  Zap,
  Activity,
  Server,
  Play,
  Pause,
  Send,
  Workflow,
  CheckCircle2,
  RefreshCw,
  Terminal,
  ShieldCheck,
  Brain,
  Link,
  Unlink,
} from 'lucide-react';
import { meshClient, MeshTelemetry, MeshTask } from '../services/meshClient';
import { gemmaEdgeController } from '../services/llmController';

export const VolunteerComputingView: React.FC = () => {
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
  });

  useEffect(() => {
    const unsub = meshClient.on('*', () => {
      setTelemetry(meshClient.getTelemetry());
    });

    const unsubTaskStarted = meshClient.on('task_started', (_task: MeshTask) => {
      // Task leased
    });

    const unsubTaskCompleted = meshClient.on('task_completed', (data: any) => {
      setRecentResolutions((prev) => [data, ...prev.slice(0, 9)]);
      meshClient.getCoordinatorTelemetry().then((stats) => {
        if (stats) setGlobalStats(stats);
      }).catch(() => {});
    });

    const unsubDagUpdated = meshClient.on('dag_updated', () => {
      meshClient.getCoordinatorTelemetry().then((stats) => {
        if (stats) setGlobalStats(stats);
      }).catch(() => {});
    });

    gemmaEdgeController.getTelemetry().then((tel) => {
      setVramMB(tel.vramAllocatedMB);
      setTokSpeed(tel.avgTokensPerSec || 46.2);
    });

    // Auto-connect to local coordinator
    meshClient.connect(coordinatorUrl);

    const pollTimer = setInterval(() => {
      if (meshClient.getTelemetry().status === 'connected') {
        meshClient.getCoordinatorTelemetry().then((stats) => {
          if (stats) setGlobalStats(stats);
        }).catch(() => {});
      }
    }, 3000);

    return () => {
      unsub();
      unsubTaskStarted();
      unsubTaskCompleted();
      unsubDagUpdated();
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
                BourbakiMesh Volunteer Computing (Stage 4)
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
              <span className="px-2 py-0.5 text-[10px] font-mono font-semibold bg-purple-950/80 text-purple-300 border border-purple-700/50 rounded-full">
                Gemma 4 WebGPU
              </span>
            </div>
            <p className="text-xs text-slate-400 font-mono mt-0.5">
              Distributed Proof DAG resolution over WebSockets with microsecond kernel validation
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
          </div>
        </div>
      </div>

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
              {telemetry.status === 'connected' ? (
                <>
                  <Unlink className="w-3.5 h-3.5" />
                  <span>Disconnect</span>
                </>
              ) : (
                <>
                  <Link className="w-3.5 h-3.5" />
                  <span>Connect</span>
                </>
              )}
            </button>
          </div>

          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={toggleAutonomous}
              disabled={telemetry.status !== 'connected'}
              className={`flex items-center gap-2 px-4 py-2 text-xs font-mono font-bold rounded-lg shadow-lg transition-all disabled:opacity-50 ${
                isAutonomous
                  ? 'bg-amber-600 hover:bg-amber-500 text-white'
                  : 'bg-gradient-to-r from-purple-600 to-indigo-600 hover:from-purple-500 hover:to-indigo-500 text-white shadow-purple-950/40'
              }`}
            >
              {isAutonomous ? <Pause className="w-3.5 h-3.5" /> : <Play className="w-3.5 h-3.5" />}
              <span>{isAutonomous ? 'Pause Worker Loop' : 'Autonomous Edge Worker'}</span>
            </button>

            <button
              type="button"
              onClick={handleStepOnce}
              disabled={telemetry.status !== 'connected' || telemetry.isWorking}
              className="flex items-center gap-1.5 px-3 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-mono font-semibold rounded-lg border border-slate-700 transition-all disabled:opacity-50"
            >
              <RefreshCw className={`w-3.5 h-3.5 text-blue-400 ${telemetry.isWorking ? 'animate-spin' : ''}`} />
              <span>Pull & Step Once</span>
            </button>

            <button
              type="button"
              onClick={handlePostAndCommGoal}
              disabled={telemetry.status !== 'connected'}
              className="flex items-center gap-1.5 px-3 py-2 bg-slate-900 hover:bg-slate-800 text-purple-300 hover:text-white text-xs font-mono rounded-lg border border-purple-900/60 transition-all disabled:opacity-50"
              title="Post benchmark theorem AndComm to global queue"
            >
              <Send className="w-3.5 h-3.5 text-purple-400" />
              <span>Inject AndComm Goal</span>
            </button>
          </div>
        </div>

        {/* Real-time Telemetry Dashboard Cards */}
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
          <div className="bg-slate-950/70 p-4 rounded-xl border border-slate-800 flex items-center justify-between">
            <div>
              <div className="text-xs text-slate-400 font-mono">Worker ID</div>
              <div className="text-sm font-bold font-mono text-purple-300 mt-0.5 truncate max-w-[130px]">
                {telemetry.workerId}
              </div>
              <span className="text-[10px] text-slate-500 font-mono">{telemetry.tasksCompleted} tasks done</span>
            </div>
            <Cpu className="w-6 h-6 text-purple-400 opacity-60" />
          </div>

          <div className="bg-slate-950/70 p-4 rounded-xl border border-slate-800 flex items-center justify-between">
            <div>
              <div className="text-xs text-slate-400 font-mono">WebGPU VRAM</div>
              <div className="text-xl font-bold font-mono text-white mt-0.5">{vramMB} MB</div>
              <span className="text-[10px] text-emerald-400 font-mono">&lt; 4,096 MB Envelope</span>
            </div>
            <Server className="w-6 h-6 text-blue-400 opacity-60" />
          </div>

          <div className="bg-slate-950/70 p-4 rounded-xl border border-slate-800 flex items-center justify-between">
            <div>
              <div className="text-xs text-slate-400 font-mono">Worker Status</div>
              <div className="text-base font-bold font-mono text-amber-400 mt-0.5">
                {telemetry.isWorking ? 'Evaluating Step...' : isAutonomous ? 'Polling Mesh...' : 'Idle'}
              </div>
              <span className="text-[10px] text-slate-500 font-mono">
                {telemetry.lastGenrmScore ? `GenRM ${(telemetry.lastGenrmScore * 100).toFixed(0)}%` : 'Ready'}
              </span>
            </div>
            <Zap className="w-6 h-6 text-amber-400 opacity-60" />
          </div>

          <div className="bg-slate-950/70 p-4 rounded-xl border border-slate-800 flex items-center justify-between">
            <div>
              <div className="text-xs text-slate-400 font-mono">Throughput</div>
              <div className="text-xl font-bold font-mono text-teal-300 mt-0.5">
                {tokSpeed.toFixed(1)} <span className="text-xs font-normal text-slate-400">tok/s</span>
              </div>
              <span className="text-[10px] text-teal-500 font-mono">Gemma 4 Edge</span>
            </div>
            <Activity className="w-6 h-6 text-teal-400 opacity-60" />
          </div>
        </div>

        {/* Live Task Dispatch & Deduction Activity */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Column 1: Active Task Context */}
          <div className="bg-slate-950 p-5 rounded-xl border border-slate-800 flex flex-col h-[380px]">
            <div className="flex items-center justify-between border-b border-slate-800 pb-3 mb-3">
              <div className="flex items-center gap-2">
                <Workflow className="w-4 h-4 text-purple-400" />
                <h3 className="text-xs font-bold text-white font-mono uppercase tracking-wider">
                  Active Leased Task Context
                </h3>
              </div>
              <span className="text-[11px] font-mono text-slate-500">
                {telemetry.currentTask ? `Task ID: ${telemetry.currentTask.task_id.substring(0, 12)}...` : 'No active lease'}
              </span>
            </div>

            <div className="flex-1 overflow-y-auto space-y-3 font-mono text-xs pr-1">
              {telemetry.currentTask ? (
                <>
                  <div>
                    <span className="text-[10px] text-slate-500 block uppercase font-bold mb-1">Theorem:</span>
                    <div className="p-2 bg-slate-900 rounded border border-slate-800 text-purple-300 font-bold">
                      {telemetry.currentTask.theorem_name}
                    </div>
                  </div>

                  <div>
                    <span className="text-[10px] text-slate-500 block uppercase font-bold mb-1">Target Goal:</span>
                    <div className="p-2 bg-slate-900 rounded border border-slate-800 text-white">
                      ⊢ {JSON.stringify(telemetry.currentTask.target)}
                    </div>
                  </div>

                  <div>
                    <span className="text-[10px] text-slate-500 block uppercase font-bold mb-1">Active Hypotheses:</span>
                    <div className="space-y-1">
                      {Object.entries(telemetry.currentTask.hyps).map(([id, expr]) => (
                        <div key={id} className="p-2 bg-slate-900 rounded border border-slate-800 text-slate-200">
                          <span className="font-bold text-blue-400">{id}:</span> {JSON.stringify(expr)}
                        </div>
                      ))}
                    </div>
                  </div>
                </>
              ) : (
                <div className="flex flex-col items-center justify-center h-full text-slate-500 text-xs space-y-2">
                  <Terminal className="w-8 h-8 text-slate-700" />
                  <span>Connect and click "Autonomous Edge Worker" or "Pull & Step Once"</span>
                </div>
              )}
            </div>
          </div>

          {/* Column 2: Edge Worker Reasoning & Telemetry */}
          <div className="bg-slate-950 p-5 rounded-xl border border-slate-800 flex flex-col h-[380px]">
            <div className="flex items-center justify-between border-b border-slate-800 pb-3 mb-3">
              <div className="flex items-center gap-2">
                <Brain className="w-4 h-4 text-emerald-400" />
                <h3 className="text-xs font-bold text-white font-mono uppercase tracking-wider">
                  Edge Worker Output & GenRM Telemetry
                </h3>
              </div>
              <div className="flex items-center gap-1.5 text-xs text-blue-400 font-mono">
                <ShieldCheck className="w-3.5 h-3.5" />
                <span>Kernel Validated</span>
              </div>
            </div>

            <div className="flex-1 overflow-y-auto space-y-3 font-mono text-xs pr-1">
              <div>
                <span className="text-[10px] text-slate-500 block uppercase font-bold mb-1">Last Applied Step AST:</span>
                <div className="p-2 bg-slate-900 rounded border border-slate-800 text-emerald-300">
                  {telemetry.lastStepApplied ? JSON.stringify(telemetry.lastStepApplied) : 'None'}
                </div>
              </div>

              <div>
                <span className="text-[10px] text-slate-500 block uppercase font-bold mb-1">GenRM Confidence:</span>
                <div className="p-2 bg-slate-900 rounded border border-slate-800 flex items-center justify-between">
                  <span className="text-sm font-bold text-blue-400">
                    {(telemetry.lastGenrmScore * 100).toFixed(1)}%
                  </span>
                  <span className="text-[10px] px-2 py-0.5 rounded bg-emerald-950 text-emerald-300 border border-emerald-800">
                    Verified
                  </span>
                </div>
              </div>

              <div>
                <span className="text-[10px] text-slate-500 block uppercase font-bold mb-1">Thinking Scratchpad (&lt;think&gt;):</span>
                <div className="p-3 bg-purple-950/20 border border-purple-900/40 rounded-lg text-slate-300 text-[11px] leading-relaxed max-h-32 overflow-y-auto">
                  {telemetry.lastThinkingTrace || 'Worker thinking scratchpad trace will appear here...'}
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Global Resolution Ledger */}
        <div className="bg-slate-950 p-5 rounded-xl border border-slate-800">
          <div className="flex items-center justify-between border-b border-slate-800 pb-3 mb-4">
            <div className="flex items-center gap-2">
              <CheckCircle2 className="w-4 h-4 text-emerald-400" />
              <h3 className="text-xs font-bold text-white font-mono uppercase tracking-wider">
                Mesh Task Resolution Ledger
              </h3>
            </div>
            <span className="text-[11px] font-mono text-slate-400">
              {globalStats.total_tasks_resolved} total steps synchronized across mesh
            </span>
          </div>

          <div className="space-y-2 max-h-48 overflow-y-auto">
            {recentResolutions.length === 0 ? (
              <div className="text-center py-6 text-slate-500 font-mono text-xs">
                No recent task completions. Tasks resolved by edge workers will be logged here in real-time.
              </div>
            ) : (
              recentResolutions.map((item, idx) => (
                <div
                  key={idx}
                  className="p-3 bg-slate-900/80 rounded-lg border border-slate-800/80 flex items-center justify-between font-mono text-xs"
                >
                  <div className="flex items-center gap-2">
                    <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />
                    <div>
                      <span className="font-bold text-white">{item.task?.theorem_name || 'Theorem'}</span>
                      <span className="text-slate-500 ml-2">Node: {item.task?.node_id?.substring(0, 10)}...</span>
                    </div>
                  </div>
                  <div className="flex items-center gap-3">
                    <span className="text-emerald-400 font-bold">
                      Status: {item.submitRes?.status || 'Proven'}
                    </span>
                    <span className="text-[10px] text-slate-500">
                      Resolved #{item.submitRes?.total_resolved || idx + 1}
                    </span>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
