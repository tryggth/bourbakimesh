import React, { useState, useEffect } from 'react';
import { Target, Send, CheckCircle2, Flame, RefreshCw, BookOpen, Layers } from 'lucide-react';

export interface TargetInfo {
  name: string;
  proposition: string;
  lean_code: string;
  priority: number;
  status: string;
  dedicated_sims: number;
  open_subgoals: number;
  timestamp: number;
}

const PRESET_LEMMAS = [
  {
    name: 'Mathlib.Logic.And.intro',
    proposition: 'A -> B -> A ∧ B',
    lean_code: 'theorem and_intro (a : A) (b : B) : A ∧ B :=\n  And.intro a b',
    priority: 100,
  },
  {
    name: 'Mathlib.Logic.ModusPonens',
    proposition: 'P -> (P -> Q) -> Q',
    lean_code: 'theorem modus_ponens (p : P) (f : P -> Q) : Q :=\n  f p',
    priority: 120,
  },
  {
    name: 'Mathlib.Order.Basic.le_trans',
    proposition: 'a ≤ b -> b ≤ c -> a ≤ c',
    lean_code: 'theorem le_trans (h1 : a ≤ b) (h2 : b ≤ c) : a ≤ c := by sorry',
    priority: 150,
  },
  {
    name: 'Mathlib.Algebra.Group.mul_left_inv',
    proposition: 'a⁻¹ * a = 1',
    lean_code: 'theorem mul_left_inv (a : G) : a⁻¹ * a = 1 := by sorry',
    priority: 200,
  },
];

interface TargetManagerProps {
  currentTarget?: TargetInfo;
  onTargetChanged?: (target: TargetInfo) => void;
}

export const TargetManager: React.FC<TargetManagerProps> = ({
  currentTarget: initialTarget,
  onTargetChanged,
}) => {
  const [target, setTarget] = useState<TargetInfo>(
    initialTarget || {
      name: 'Mathlib.Logic.And.intro',
      proposition: 'A -> B -> A ∧ B',
      lean_code: 'theorem and_intro (a : A) (b : B) : A ∧ B :=\n  And.intro a b',
      priority: 100,
      status: 'active',
      dedicated_sims: 1420,
      open_subgoals: 1,
      timestamp: Date.now() / 1000,
    }
  );

  const [inputName, setInputName] = useState(target.name);
  const [inputLean, setInputLean] = useState(target.lean_code);
  const [priority, setPriority] = useState<number>(100);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isExpanded, setIsExpanded] = useState(false);
  const [successToast, setSuccessToast] = useState(false);

  // Fetch current active target on mount
  useEffect(() => {
    fetch('/api/target/current')
      .then((res) => res.json())
      .then((data: TargetInfo) => {
        if (data.name) {
          setTarget(data);
          setInputName(data.name);
          setInputLean(data.lean_code);
          setPriority(data.priority);
        }
      })
      .catch(() => {});
  }, []);

  const handleSelectPreset = (preset: (typeof PRESET_LEMMAS)[0]) => {
    setInputName(preset.name);
    setInputLean(preset.lean_code);
    setPriority(preset.priority);
  };

  const handleSetTarget = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    try {
      const res = await fetch('/api/target/set', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: inputName,
          lean_code: inputLean,
          priority,
        }),
      });
      if (res.ok) {
        const resData = await res.json();
        if (resData.target) {
          setTarget(resData.target);
          if (onTargetChanged) onTargetChanged(resData.target);
          setSuccessToast(true);
          setTimeout(() => setSuccessToast(false), 3000);
          setIsExpanded(false);
        }
      }
    } catch (err) {
      console.error('Failed to set target theorem:', err);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="bg-slate-900/90 border-b border-slate-800 backdrop-blur px-6 py-3 transition-all duration-200">
      {/* Top Summary Banner */}
      <div className="flex flex-wrap items-center justify-between gap-4">
        {/* Left: Active Objective Focus Badge */}
        <div className="flex items-center gap-3 min-w-0">
          <div className="p-2 bg-blue-600/20 border border-blue-500/40 rounded-lg text-blue-400 shrink-0">
            <Target className="w-5 h-5 animate-pulse" />
          </div>
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <span className="text-[10px] font-bold uppercase tracking-wider px-2 py-0.5 rounded bg-emerald-950/80 text-emerald-400 border border-emerald-800/80 flex items-center gap-1">
                <Flame className="w-3 h-3 text-emerald-400" />
                Active Swarm Focus
              </span>
              <span className="text-xs text-slate-400 font-mono">Priority: {target.priority}</span>
            </div>
            <div className="flex items-baseline gap-2 truncate">
              <span className="font-semibold text-sm text-white truncate">{target.name}</span>
              <span className="text-xs text-blue-300 font-mono hidden sm:inline truncate">
                {target.proposition}
              </span>
            </div>
          </div>
        </div>

        {/* Right: Quick Action Controls & Stats */}
        <div className="flex items-center gap-3 shrink-0">
          <div className="hidden md:flex items-center gap-4 px-3 py-1.5 bg-slate-950/60 border border-slate-800/80 rounded-lg text-xs font-mono text-slate-300">
            <div className="flex items-center gap-1.5">
              <Layers className="w-3.5 h-3.5 text-blue-400" />
              <span>Sims: {target.dedicated_sims.toLocaleString()}</span>
            </div>
            <div className="flex items-center gap-1.5">
              <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" />
              <span>Open Holes: {target.open_subgoals}</span>
            </div>
          </div>

          <button
            onClick={() => setIsExpanded(!isExpanded)}
            className={`px-3 py-1.5 rounded-lg text-xs font-semibold border transition-colors flex items-center gap-1.5 ${
              isExpanded
                ? 'bg-slate-800 text-white border-slate-700 shadow'
                : 'bg-blue-600 hover:bg-blue-500 text-white border-blue-500 shadow'
            }`}
          >
            <Target className="w-3.5 h-3.5" />
            <span>{isExpanded ? 'Close Objective Bar' : 'Set Swarm Target'}</span>
          </button>
        </div>
      </div>

      {/* Success Notification Toast */}
      {successToast && (
        <div className="mt-2 px-3 py-1.5 bg-emerald-950/90 border border-emerald-700/80 rounded-lg text-xs text-emerald-300 flex items-center gap-2 animate-in fade-in">
          <CheckCircle2 className="w-4 h-4 text-emerald-400" />
          <span>Swarm objective successfully updated! Directives broadcast to P2P mesh.</span>
        </div>
      )}

      {/* Expanded Target Injection Drawer */}
      {isExpanded && (
        <form
          onSubmit={handleSetTarget}
          className="mt-4 pt-4 border-t border-slate-800/80 grid grid-cols-1 lg:grid-cols-3 gap-4 animate-in slide-in-from-top-2 duration-200"
        >
          {/* Preset Selector */}
          <div className="space-y-2">
            <label className="text-xs font-semibold text-slate-300 flex items-center gap-1.5">
              <BookOpen className="w-3.5 h-3.5 text-blue-400" />
              <span>Mathlib Curriculum Presets</span>
            </label>
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-1 gap-1.5">
              {PRESET_LEMMAS.map((preset) => (
                <button
                  key={preset.name}
                  type="button"
                  onClick={() => handleSelectPreset(preset)}
                  className={`text-left p-2 rounded-lg border text-xs transition-colors ${
                    inputName === preset.name
                      ? 'bg-blue-950/60 border-blue-600 text-blue-200'
                      : 'bg-slate-950/40 border-slate-800 text-slate-400 hover:text-slate-200 hover:border-slate-700'
                  }`}
                >
                  <div className="font-semibold text-[11px] truncate">{preset.name}</div>
                  <div className="text-[10px] text-slate-500 font-mono truncate">
                    {preset.proposition}
                  </div>
                </button>
              ))}
            </div>
          </div>

          {/* Custom Theorem Statement */}
          <div className="lg:col-span-2 space-y-3">
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
              <div className="sm:col-span-2">
                <label className="text-xs font-semibold text-slate-300 block mb-1">
                  Theorem Identifier
                </label>
                <input
                  type="text"
                  value={inputName}
                  onChange={(e) => setInputName(e.target.value)}
                  placeholder="e.g. Mathlib.Algebra.Group.inv_mul_cancel"
                  className="w-full px-3 py-1.5 bg-slate-950 border border-slate-800 rounded-lg text-xs font-mono text-slate-200 focus:outline-none focus:border-blue-500"
                  required
                />
              </div>

              <div>
                <label className="text-xs font-semibold text-slate-300 block mb-1">
                  Swarm Priority
                </label>
                <select
                  value={priority}
                  onChange={(e) => setPriority(Number(e.target.value))}
                  className="w-full px-3 py-1.5 bg-slate-950 border border-slate-800 rounded-lg text-xs font-mono text-slate-200 focus:outline-none focus:border-blue-500"
                >
                  <option value={50}>Low (50)</option>
                  <option value={100}>Normal (100)</option>
                  <option value={150}>High (150)</option>
                  <option value={200}>Critical / Urgent (200)</option>
                </select>
              </div>
            </div>

            <div>
              <label className="text-xs font-semibold text-slate-300 block mb-1">
                Lean 4 Statement & Signature
              </label>
              <textarea
                value={inputLean}
                onChange={(e) => setInputLean(e.target.value)}
                rows={3}
                placeholder="theorem custom_lemma (a : A) (b : B) : A ∧ B := by sorry"
                className="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-xs font-mono text-slate-200 focus:outline-none focus:border-blue-500"
                required
              />
            </div>

            <div className="flex justify-end gap-2">
              <button
                type="button"
                onClick={() => setIsExpanded(false)}
                className="px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded-lg text-xs font-semibold transition-colors"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={isSubmitting}
                className="px-4 py-1.5 bg-blue-600 hover:bg-blue-500 disabled:bg-blue-800 text-white rounded-lg text-xs font-semibold shadow transition-colors flex items-center gap-1.5"
              >
                {isSubmitting ? (
                  <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                ) : (
                  <Send className="w-3.5 h-3.5" />
                )}
                <span>Broadcast Swarm Directive</span>
              </button>
            </div>
          </div>
        </form>
      )}
    </div>
  );
};
