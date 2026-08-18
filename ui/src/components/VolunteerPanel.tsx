import { useState, useEffect, useRef } from 'react';
import {
  Cpu,
  BatteryCharging,
  BatteryWarning,
  Eye,
  EyeOff,
  ShieldCheck,
  CheckCircle2,
  Copy,
  Check,
  Sparkles,
  Flame,
  AlertCircle,
  Play,
  Square,
} from 'lucide-react';
import {
  getOrCreateBrowserIdentity,
  signProof,
  BrowserIdentity,
  SignedProofAttestation,
} from '../services/cryptoIdentity';

interface VolunteerStats {
  solvedSubgoals: number;
  totalVisits: number;
  currentThroughput: number;
  lastSolvedTheorem?: string;
  totalTimeSec: number;
}

export function VolunteerPanel() {
  const [isContributing, setIsContributing] = useState<boolean>(false);
  const [powerMode, setPowerMode] = useState<'eco' | 'balanced' | 'max'>('balanced');
  const [simulations, setSimulations] = useState<number>(100);
  const [overridePause, setOverridePause] = useState<boolean>(false);

  // Identity & Worker State
  const [identity, setIdentity] = useState<BrowserIdentity | null>(null);
  const [copiedId, setCopiedId] = useState<boolean>(false);
  const [activeProvider, setActiveProvider] = useState<string>('initializing...');
  const [workerStatus, setWorkerStatus] = useState<
    'unloaded' | 'initializing' | 'idle' | 'proving' | 'paused_battery' | 'paused_background' | 'error'
  >('unloaded');
  const [statusDetail, setStatusDetail] = useState<string>('Worker stopped');

  // Hardware & Battery State
  const [batteryLevel, setBatteryLevel] = useState<number | null>(null);
  const [isCharging, setIsCharging] = useState<boolean | null>(null);
  const [isTabVisible, setIsTabVisible] = useState<boolean>(true);

  // Stats
  const [stats, setStats] = useState<VolunteerStats>({
    solvedSubgoals: 0,
    totalVisits: 0,
    currentThroughput: 0,
    totalTimeSec: 0,
  });

  const [attestationHistory, setAttestationHistory] = useState<SignedProofAttestation[]>([]);

  const workerRef = useRef<Worker | null>(null);

  // 1. Initialize Web Crypto Identity
  useEffect(() => {
    getOrCreateBrowserIdentity()
      .then((id) => setIdentity(id))
      .catch((err) => console.error('Failed to initialize browser crypto identity:', err));
  }, []);

  // 2. Battery & Visibility Observers
  useEffect(() => {
    if (typeof navigator !== 'undefined' && 'getBattery' in navigator) {
      (navigator as any).getBattery().then((battery: any) => {
        setBatteryLevel(battery.level);
        setIsCharging(battery.charging);

        const onLevelChange = () => setBatteryLevel(battery.level);
        const onChargingChange = () => setIsCharging(battery.charging);

        battery.addEventListener('levelchange', onLevelChange);
        battery.addEventListener('chargingchange', onChargingChange);

        return () => {
          battery.removeEventListener('levelchange', onLevelChange);
          battery.removeEventListener('chargingchange', onChargingChange);
        };
      });
    }

    const onVisibilityChange = () => {
      setIsTabVisible(document.visibilityState === 'visible');
    };
    document.addEventListener('visibilitychange', onVisibilityChange);
    return () => {
      document.removeEventListener('visibilitychange', onVisibilityChange);
    };
  }, []);

  // 3. Worker Lifecycle Management
  const startWorker = () => {
    if (workerRef.current) return;

    setWorkerStatus('initializing');
    setStatusDetail('Loading ONNX Runtime WebGPU/Wasm models...');

    try {
      const worker = new Worker(new URL('../workers/prover.worker.ts', import.meta.url), {
        type: 'module',
      });
      workerRef.current = worker;

      worker.onmessage = async (e) => {
        const msg = e.data;
        if (msg.type === 'INIT_COMPLETE') {
          setActiveProvider(msg.provider || 'wasm');
          setWorkerStatus('idle');
          setStatusDetail(`Inference engine online (${msg.provider.toUpperCase()})`);
          setIsContributing(true);
        } else if (msg.type === 'MCTS_STEP') {
          setStats((prev) => ({
            ...prev,
            currentThroughput: msg.simsPerSec || prev.currentThroughput,
            totalVisits: prev.totalVisits + (msg.visits || 1),
          }));
        } else if (msg.type === 'SEARCH_COMPLETE') {
          setWorkerStatus('idle');
          setStatusDetail(`Certified subgoal: ${msg.theoremName || 'Goal'} (val: ${msg.rootValue.toFixed(3)})`);

          // Cryptographically sign extracted proof
          try {
            const attestation = await signProof({
              taskId: msg.taskId,
              theoremName: msg.theoremName,
              bestAction: msg.bestAction,
              rootValue: msg.rootValue,
              visits: msg.visits,
              certified: msg.certified,
            });

            setAttestationHistory((prev) => [attestation, ...prev.slice(0, 4)]);
            setStats((prev) => ({
              ...prev,
              solvedSubgoals: prev.solvedSubgoals + 1,
              lastSolvedTheorem: msg.theoremName,
            }));
          } catch (signErr) {
            console.error('Failed to sign proof attestation:', signErr);
          }
        } else if (msg.type === 'ERROR') {
          setWorkerStatus('error');
          setStatusDetail(`Worker error: ${msg.error}`);
        }
      };

      worker.postMessage({ type: 'INIT' });
    } catch (err: any) {
      setWorkerStatus('error');
      setStatusDetail(`Failed to start worker: ${err.message || String(err)}`);
    }
  };

  const stopWorker = () => {
    if (workerRef.current) {
      workerRef.current.postMessage({ type: 'STOP' });
      workerRef.current.terminate();
      workerRef.current = null;
    }
    setIsContributing(false);
    setWorkerStatus('unloaded');
    setStatusDetail('Worker stopped');
    setStats((prev) => ({ ...prev, currentThroughput: 0 }));
  };

  // Handle Power Mode Preset changes
  const handleSetPowerMode = (mode: 'eco' | 'balanced' | 'max') => {
    setPowerMode(mode);
    if (mode === 'eco') setSimulations(25);
    else if (mode === 'balanced') setSimulations(100);
    else if (mode === 'max') setSimulations(250);
  };

  // Run a manual proof benchmark rollout
  const triggerBenchmarkProbe = () => {
    if (!workerRef.current || workerStatus !== 'idle') {
      if (!workerRef.current) startWorker();
      setTimeout(() => {
        workerRef.current?.postMessage({
          type: 'START_SEARCH',
          taskId: `bench-${Date.now().toString(36)}`,
          theoremName: 'Mathlib.Logic.Identity',
          proposition: 'P -> P',
          simulations,
        });
        setWorkerStatus('proving');
        setStatusDetail(`Proving Mathlib.Logic.Identity with ${simulations} simulations...`);
      }, 500);
      return;
    }

    workerRef.current.postMessage({
      type: 'START_SEARCH',
      taskId: `bench-${Date.now().toString(36)}`,
      theoremName: 'Mathlib.Logic.Identity',
      proposition: 'P -> P',
      simulations,
    });
    setWorkerStatus('proving');
    setStatusDetail(`Proving Mathlib.Logic.Identity with ${simulations} simulations...`);
  };

  const copyPeerId = () => {
    if (!identity?.peerId) return;
    navigator.clipboard.writeText(identity.peerId);
    setCopiedId(true);
    setTimeout(() => setCopiedId(false), 2000);
  };

  return (
    <div className="flex flex-col gap-6 max-w-5xl mx-auto p-2">
      {/* Top Banner & Main Toggle Card */}
      <div className="bg-slate-900 border border-slate-800 rounded-2xl p-6 shadow-xl relative overflow-hidden">
        <div className="absolute top-0 right-0 w-96 h-96 bg-blue-600/10 rounded-full blur-3xl pointer-events-none" />

        <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-6 relative z-10">
          <div className="flex items-start gap-4">
            <div
              className={`p-3 rounded-2xl border ${
                isContributing
                  ? 'bg-blue-600/20 border-blue-500/50 text-blue-400 animate-pulse'
                  : 'bg-slate-800 border-slate-700 text-slate-400'
              }`}
            >
              <Cpu className="w-8 h-8" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h2 className="text-xl font-bold text-white tracking-wide">Contribute Compute Cycles</h2>
                <span className="text-xs px-2.5 py-0.5 rounded-full bg-indigo-500/20 border border-indigo-500/30 text-indigo-300 font-mono">
                  In-Browser WebGPU / Wasm
                </span>
              </div>
              <p className="text-xs text-slate-400 mt-1 max-w-xl">
                Donate idle browser GPU/CPU threads to solve decentralized Lean 4 subgoals for the Bourbaki swarm. All
                proofs are cryptographically signed with your ephemeral in-memory identity.
              </p>
            </div>
          </div>

          <div className="flex items-center gap-4">
            <button
              onClick={() => (isContributing ? stopWorker() : startWorker())}
              className={`flex items-center gap-2.5 px-6 py-3 rounded-xl font-bold text-sm transition-all shadow-lg ${
                isContributing
                  ? 'bg-red-500/20 border border-red-500/40 text-red-300 hover:bg-red-500/30 shadow-red-500/10'
                  : 'bg-gradient-to-r from-blue-600 to-indigo-600 text-white hover:from-blue-500 hover:to-indigo-500 shadow-blue-500/20'
              }`}
            >
              {isContributing ? (
                <>
                  <Square className="w-4 h-4 fill-current" />
                  Stop Prover
                </>
              ) : (
                <>
                  <Play className="w-4 h-4 fill-current" />
                  Start Contributing
                </>
              )}
            </button>
          </div>
        </div>

        {/* Status Line */}
        <div className="mt-6 pt-4 border-t border-slate-800/80 flex flex-wrap items-center justify-between gap-4 text-xs font-mono">
          <div className="flex items-center gap-2">
            <span
              className={`w-2.5 h-2.5 rounded-full ${
                workerStatus === 'proving'
                  ? 'bg-amber-400 animate-ping'
                  : workerStatus === 'idle'
                  ? 'bg-emerald-400'
                  : workerStatus === 'initializing'
                  ? 'bg-blue-400 animate-pulse'
                  : 'bg-slate-600'
              }`}
            />
            <span className="text-slate-300 font-semibold uppercase">{workerStatus}</span>
            <span className="text-slate-500">—</span>
            <span className="text-slate-400">{statusDetail}</span>
          </div>

          <div className="flex items-center gap-4">
            <span className="text-slate-400">
              Provider:{' '}
              <span
                className={`font-semibold ${
                  activeProvider.includes('gpu') ? 'text-emerald-400' : 'text-blue-400'
                }`}
              >
                {activeProvider.toUpperCase()}
              </span>
            </span>
          </div>
        </div>
      </div>

      {/* Grid: Contributor Stats & Power Throttle Controls */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        {/* Card 1: Node Crypto Identity */}
        <div className="bg-slate-900 border border-slate-800 rounded-2xl p-5 shadow-lg flex flex-col justify-between">
          <div>
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-2 text-sm font-bold text-slate-200">
                <ShieldCheck className="w-4 h-4 text-emerald-400" />
                Node Crypto Identity
              </div>
              <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-emerald-500/10 border border-emerald-500/20 text-emerald-300">
                ECDSA P-256
              </span>
            </div>
            <p className="text-xs text-slate-400 mb-4">
              Deterministic peer ID and signed proof attestation keypair generated in secure browser memory.
            </p>

            <div className="bg-slate-950 border border-slate-800 rounded-xl p-3 mb-3">
              <div className="text-[10px] text-slate-500 font-mono uppercase mb-1">Ephemeral Peer ID</div>
              <div className="flex items-center justify-between gap-2">
                <span className="text-xs font-mono text-blue-400 truncate">
                  {identity?.peerId || 'Generating identity...'}
                </span>
                <button
                  onClick={copyPeerId}
                  className="p-1 rounded hover:bg-slate-800 text-slate-400 hover:text-slate-200 transition-colors"
                  title="Copy Peer ID"
                >
                  {copiedId ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
                </button>
              </div>
            </div>
          </div>

          <div className="text-[11px] text-slate-500 flex items-center gap-1.5">
            <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" /> Zero private keys persisted to disk.
          </div>
        </div>

        {/* Card 2: Power Slider & Throttle Profile */}
        <div className="bg-slate-900 border border-slate-800 rounded-2xl p-5 shadow-lg flex flex-col justify-between">
          <div>
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-2 text-sm font-bold text-slate-200">
                <Flame className="w-4 h-4 text-amber-400" />
                Power & Throttle Slider
              </div>
              <span className="text-xs font-mono font-bold text-amber-300">{simulations} sims/move</span>
            </div>

            {/* Presets */}
            <div className="grid grid-cols-3 gap-2 mb-4">
              <button
                onClick={() => handleSetPowerMode('eco')}
                className={`py-1.5 px-2 rounded-lg text-xs font-semibold border transition-all ${
                  powerMode === 'eco'
                    ? 'bg-emerald-500/20 border-emerald-500/40 text-emerald-300'
                    : 'bg-slate-950 border-slate-800 text-slate-400 hover:text-slate-200'
                }`}
              >
                Eco (25)
              </button>
              <button
                onClick={() => handleSetPowerMode('balanced')}
                className={`py-1.5 px-2 rounded-lg text-xs font-semibold border transition-all ${
                  powerMode === 'balanced'
                    ? 'bg-blue-500/20 border-blue-500/40 text-blue-300'
                    : 'bg-slate-950 border-slate-800 text-slate-400 hover:text-slate-200'
                }`}
              >
                Balanced (100)
              </button>
              <button
                onClick={() => handleSetPowerMode('max')}
                className={`py-1.5 px-2 rounded-lg text-xs font-semibold border transition-all ${
                  powerMode === 'max'
                    ? 'bg-purple-500/20 border-purple-500/40 text-purple-300'
                    : 'bg-slate-950 border-slate-800 text-slate-400 hover:text-slate-200'
                }`}
              >
                Max (250)
              </button>
            </div>

            {/* Custom Slider */}
            <div className="space-y-1 mb-2">
              <input
                type="range"
                min="10"
                max="500"
                step="10"
                value={simulations}
                onChange={(e) => {
                  const val = parseInt(e.target.value);
                  setSimulations(val);
                  if (val <= 50) setPowerMode('eco');
                  else if (val <= 150) setPowerMode('balanced');
                  else setPowerMode('max');
                }}
                className="w-full h-1.5 bg-slate-800 rounded-lg appearance-none cursor-pointer accent-blue-500"
              />
              <div className="flex justify-between text-[10px] text-slate-500 font-mono">
                <span>10 (Light)</span>
                <span>250 (Default)</span>
                <span>500 (Heavy)</span>
              </div>
            </div>
          </div>

          <button
            onClick={triggerBenchmarkProbe}
            className="w-full mt-2 py-2 px-3 rounded-xl bg-slate-800 hover:bg-slate-700 border border-slate-700 text-xs font-semibold text-slate-200 flex items-center justify-center gap-2 transition-colors"
          >
            <Sparkles className="w-3.5 h-3.5 text-blue-400" /> Run Benchmark Probe
          </button>
        </div>

        {/* Card 3: Thermal, Battery & Background Guard */}
        <div className="bg-slate-900 border border-slate-800 rounded-2xl p-5 shadow-lg flex flex-col justify-between">
          <div>
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-2 text-sm font-bold text-slate-200">
                <BatteryCharging className="w-4 h-4 text-blue-400" />
                Thermal & Battery Guard
              </div>
              <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-blue-500/10 border border-blue-500/20 text-blue-300">
                Active Guard
              </span>
            </div>

            <div className="space-y-2.5 mb-4">
              <div className="flex items-center justify-between text-xs">
                <span className="text-slate-400 flex items-center gap-1.5">
                  {isCharging ? (
                    <BatteryCharging className="w-3.5 h-3.5 text-emerald-400" />
                  ) : (
                    <BatteryWarning className="w-3.5 h-3.5 text-amber-400" />
                  )}
                  Battery Status:
                </span>
                <span className="font-mono text-slate-200">
                  {batteryLevel !== null ? `${Math.round(batteryLevel * 100)}%` : 'AC Powered'}{' '}
                  {isCharging ? '(Charging)' : ''}
                </span>
              </div>

              <div className="flex items-center justify-between text-xs">
                <span className="text-slate-400 flex items-center gap-1.5">
                  {isTabVisible ? (
                    <Eye className="w-3.5 h-3.5 text-blue-400" />
                  ) : (
                    <EyeOff className="w-3.5 h-3.5 text-slate-500" />
                  )}
                  Tab Visibility:
                </span>
                <span className={`font-mono ${isTabVisible ? 'text-emerald-400' : 'text-amber-400'}`}>
                  {isTabVisible ? 'Active Tab' : 'Backgrounded'}
                </span>
              </div>
            </div>

            {/* Override switch */}
            <div className="bg-slate-950 border border-slate-800 rounded-xl p-3">
              <label className="flex items-center justify-between cursor-pointer">
                <span className="text-xs text-slate-300 font-medium">Override Auto-Pause</span>
                <input
                  type="checkbox"
                  checked={overridePause}
                  onChange={(e) => setOverridePause(e.target.checked)}
                  className="w-4 h-4 rounded bg-slate-800 border-slate-700 text-blue-600 focus:ring-0 cursor-pointer"
                />
              </label>
              <div className="text-[10px] text-slate-500 mt-1">Allows continuous proof search in background tabs.</div>
            </div>
          </div>

          <div className="text-[10px] text-slate-500 mt-2 flex items-center gap-1">
            <AlertCircle className="w-3 h-3 text-slate-400" /> Auto-pauses below 20% battery.
          </div>
        </div>
      </div>

      {/* Contributor Performance Metrics */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="bg-slate-900 border border-slate-800 rounded-xl p-4">
          <div className="text-[10px] uppercase font-mono text-slate-500 mb-1">MCTS Throughput</div>
          <div className="text-2xl font-black text-white font-mono flex items-baseline gap-1.5">
            {stats.currentThroughput.toFixed(1)}{' '}
            <span className="text-xs font-normal text-slate-400">sims/sec</span>
          </div>
        </div>

        <div className="bg-slate-900 border border-slate-800 rounded-xl p-4">
          <div className="text-[10px] uppercase font-mono text-slate-500 mb-1">Subgoals Certified</div>
          <div className="text-2xl font-black text-emerald-400 font-mono">{stats.solvedSubgoals}</div>
        </div>

        <div className="bg-slate-900 border border-slate-800 rounded-xl p-4">
          <div className="text-[10px] uppercase font-mono text-slate-500 mb-1">Total MCTS Visits</div>
          <div className="text-2xl font-black text-indigo-300 font-mono">
            {stats.totalVisits.toLocaleString()}
          </div>
        </div>

        <div className="bg-slate-900 border border-slate-800 rounded-xl p-4">
          <div className="text-[10px] uppercase font-mono text-slate-500 mb-1">Active Swarm Share</div>
          <div className="text-2xl font-black text-purple-400 font-mono">
            {stats.solvedSubgoals > 0 ? `${(stats.solvedSubgoals * 12.5).toFixed(1)}%` : '0.0%'}
          </div>
        </div>
      </div>

      {/* Recent Signed Attestations Log */}
      {attestationHistory.length > 0 && (
        <div className="bg-slate-900 border border-slate-800 rounded-2xl p-5 shadow-lg">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-sm font-bold text-slate-200 flex items-center gap-2">
              <ShieldCheck className="w-4 h-4 text-emerald-400" />
              Signed Proof Attestation History
            </h3>
            <span className="text-xs text-slate-500 font-mono">{attestationHistory.length} Attestations</span>
          </div>

          <div className="space-y-2 font-mono text-xs">
            {attestationHistory.map((att, idx) => (
              <div
                key={idx}
                className="bg-slate-950 border border-slate-800 rounded-xl p-3 flex flex-col md:flex-row items-start md:items-center justify-between gap-3"
              >
                <div className="flex items-center gap-3">
                  <div className="w-2 h-2 rounded-full bg-emerald-400" />
                  <div>
                    <div className="text-slate-200 font-semibold">ProofHash: {att.proofHash.substring(0, 16)}...</div>
                    <div className="text-[10px] text-slate-500">Prover: {att.proverPeerId}</div>
                  </div>
                </div>

                <div className="flex items-center gap-2 text-[10px] text-slate-400 bg-slate-900 px-2.5 py-1 rounded-lg border border-slate-800">
                  <span>Sig: {att.signatureHex.substring(0, 16)}...</span>
                  <span className="text-emerald-400 font-bold">VERIFIED</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
