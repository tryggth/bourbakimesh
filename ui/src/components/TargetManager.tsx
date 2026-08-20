import React, { useState } from 'react';
import { Target, Send, CheckCircle2, Flame, RefreshCw, BookOpen, Layers } from 'lucide-react';
import { meshClient } from '../services/meshClient';

export interface TargetInfo {
  name: string;
  proposition: string;
  lean_code: string;
  priority: number;
  status: string;
  dedicated_searches: number;
  open_subgoals: number;
  timestamp: number;
  target_type?: any;
}

export const PRESET_LEMMAS: Array<{
  name: string;
  proposition: string;
  lean_code: string;
  priority: number;
  target_type: any;
}> = [
  {
    name: 'And.swap',
    proposition: 'A ∧ B → B ∧ A',
    lean_code: 'theorem and_swap (h : A ∧ B) : B ∧ A :=\n  ⟨h.2, h.1⟩',
    priority: 100,
    target_type: {
      Pi: [
        'u',
        { Sort: 'Type' },
        {
          Pi: [
            'A',
            { Sort: 'Prop' },
            {
              Pi: [
                'B',
                { Sort: 'Prop' },
                {
                  Pi: [
                    'h',
                    { App: [{ App: [{ Const: ['And', []] }, { BVar: 1 }] }, { BVar: 0 }] },
                    { App: [{ App: [{ Const: ['And', []] }, { BVar: 1 }] }, { BVar: 2 }] },
                  ],
                },
              ],
            },
          ],
        },
      ],
    },
  },
  {
    name: 'Or.swap',
    proposition: 'A ∨ B → B ∨ A',
    lean_code: 'theorem or_swap (h : A ∨ B) : B ∨ A :=\n  h.elim (fun a => Or.inr a) (fun b => Or.inl b)',
    priority: 110,
    target_type: {
      Pi: [
        'u',
        { Sort: 'Type' },
        {
          Pi: [
            'A',
            { Sort: 'Prop' },
            {
              Pi: [
                'B',
                { Sort: 'Prop' },
                {
                  Pi: [
                    'h',
                    { App: [{ App: [{ Const: ['Or', []] }, { BVar: 1 }] }, { BVar: 0 }] },
                    { App: [{ App: [{ Const: ['Or', []] }, { BVar: 1 }] }, { BVar: 2 }] },
                  ],
                },
              ],
            },
          ],
        },
      ],
    },
  },
  {
    name: 'Eq.symm',
    proposition: 'a = b → b = a',
    lean_code: 'theorem eq_symm (h : a = b) : b = a :=\n  h.symm',
    priority: 120,
    target_type: {
      Pi: [
        'α',
        { Sort: 'Type' },
        {
          Pi: [
            'a',
            { BVar: 0 },
            {
              Pi: [
                'b',
                { BVar: 1 },
                {
                  Pi: [
                    'h',
                    { App: [{ App: [{ App: [{ Const: ['Eq', []] }, { BVar: 2 }] }, { BVar: 1 }] }, { BVar: 0 }] },
                    { App: [{ App: [{ App: [{ Const: ['Eq', []] }, { BVar: 2 }] }, { BVar: 0 }] }, { BVar: 1 }] },
                  ],
                },
              ],
            },
          ],
        },
      ],
    },
  },
  {
    name: 'peirce_law',
    proposition: '((P → Q) → P) → P',
    lean_code: 'theorem peirce_law (h : (P → Q) → P) : P :=\n  Classical.byContradiction (fun np => np (h (fun p => False.elim (np p))))',
    priority: 150,
    target_type: {
      Pi: [
        'P',
        { Sort: 'Prop' },
        {
          Pi: [
            'Q',
            { Sort: 'Prop' },
            {
              Pi: [
                'h',
                {
                  Pi: [
                    '_',
                    { Pi: ['_', { BVar: 1 }, { BVar: 1 }] },
                    { BVar: 2 },
                  ],
                },
                { BVar: 2 },
              ],
            },
          ],
        },
      ],
    },
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
      name: 'And.swap',
      proposition: 'A ∧ B → B ∧ A',
      lean_code: 'theorem and_swap (h : A ∧ B) : B ∧ A :=\n  ⟨h.2, h.1⟩',
      priority: 100,
      status: 'active',
      dedicated_searches: 1420,
      open_subgoals: 1,
      timestamp: Date.now() / 1000,
      target_type: PRESET_LEMMAS[0].target_type,
    }
  );

  const [inputName, setInputName] = useState(target.name);
  const [inputLean, setInputLean] = useState(target.lean_code);
  const [selectedTargetType, setSelectedTargetType] = useState<any>(PRESET_LEMMAS[0].target_type);
  const [priority, setPriority] = useState<number>(100);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isExpanded, setIsExpanded] = useState(false);
  const [successToast, setSuccessToast] = useState(false);
  const [toastMessage, setToastMessage] = useState('');

  const handleSelectPreset = (preset: (typeof PRESET_LEMMAS)[0]) => {
    setInputName(preset.name);
    setInputLean(preset.lean_code);
    setSelectedTargetType(preset.target_type);
    setPriority(preset.priority);
  };

  const handleSetTarget = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    try {
      const matched = PRESET_LEMMAS.find((p) => p.name === inputName);
      const targetTypeToSend = matched?.target_type || selectedTargetType || { Sort: 'Prop' };

      const res = await meshClient.postTarget(inputName, targetTypeToSend);

      const updatedTarget: TargetInfo = {
        name: inputName,
        proposition: matched?.proposition || inputName,
        lean_code: inputLean,
        priority,
        status: 'active',
        dedicated_searches: 1,
        open_subgoals: 1,
        timestamp: Date.now() / 1000,
        target_type: targetTypeToSend,
      };

      setTarget(updatedTarget);
      if (onTargetChanged) onTargetChanged(updatedTarget);

      setToastMessage(`Swarm target "${inputName}" posted to coordinator (Task ${res?.task_id || 'queued'})`);
      setSuccessToast(true);
      setTimeout(() => setSuccessToast(false), 3500);
      setIsExpanded(false);
    } catch (err: any) {
      console.error('[TargetManager] Failed to post target via JSON-RPC:', err);
      setToastMessage(`Target submission: ${err.message || 'Coordinator offline'}`);
      setSuccessToast(true);
      setTimeout(() => setSuccessToast(false), 3500);
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
              <span>Search Nodes: {target.dedicated_searches.toLocaleString()}</span>
            </div>
            <div className="flex items-center gap-1.5">
              <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" />
              <span>Open Subgoals: {target.open_subgoals}</span>
            </div>
          </div>

          <button
            onClick={() => setIsExpanded(!isExpanded)}
            className={`px-3 py-1.5 rounded-lg text-xs font-semibold border transition-colors flex items-center gap-1.5 cursor-pointer ${
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
          <span>{toastMessage}</span>
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
              <span>Mathlib Verified CIC Presets</span>
            </label>
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-1 gap-1.5">
              {PRESET_LEMMAS.map((preset) => (
                <button
                  key={preset.name}
                  type="button"
                  onClick={() => handleSelectPreset(preset)}
                  className={`text-left p-2 rounded-lg border text-xs transition-colors cursor-pointer ${
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
                  placeholder="e.g. And.swap"
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
                  className="w-full px-3 py-1.5 bg-slate-950 border border-slate-800 rounded-lg text-xs font-mono text-slate-200 focus:outline-none focus:border-blue-500 cursor-pointer"
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
                Lean 4 Statement & Formal Spec
              </label>
              <textarea
                value={inputLean}
                onChange={(e) => setInputLean(e.target.value)}
                rows={3}
                placeholder="theorem and_swap (h : A ∧ B) : B ∧ A := ⟨h.2, h.1⟩"
                className="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-xs font-mono text-slate-200 focus:outline-none focus:border-blue-500"
                required
              />
            </div>

            <div className="flex justify-end gap-2">
              <button
                type="button"
                onClick={() => setIsExpanded(false)}
                className="px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded-lg text-xs font-semibold transition-colors cursor-pointer"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={isSubmitting}
                className="px-4 py-1.5 bg-blue-600 hover:bg-blue-500 disabled:bg-blue-800 text-white rounded-lg text-xs font-semibold shadow transition-colors flex items-center gap-1.5 cursor-pointer"
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
