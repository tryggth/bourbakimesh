import React, { useState } from 'react';
import { ProveResponse } from '../types';
import { Play, Sparkles, CheckCircle2, Clock, Loader2 } from 'lucide-react';

interface TheoremProverViewProps {
  onProve: (theoremName: string, proposition: string) => Promise<ProveResponse | null>;
  isSearching: boolean;
}

const PRESET_GOALS = [
  { name: "Mathlib.Logic.And.intro", prop: "A -> B -> A ∧ B" },
  { name: "Mathlib.Logic.Identity", prop: "P -> P" },
  { name: "Mathlib.Logic.ModusPonens", prop: "P -> (P -> Q) -> Q" },
  { name: "Mathlib.Logic.Transitivity", prop: "(P -> Q) -> (Q -> R) -> (P -> R)" },
  { name: "Mathlib.Logic.CutLemma", prop: "P -> P" },
];

export const TheoremProverView: React.FC<TheoremProverViewProps> = ({ onProve, isSearching }) => {
  const [theoremName, setTheoremName] = useState(PRESET_GOALS[0].name);
  const [proposition, setProposition] = useState(PRESET_GOALS[0].prop);
  const [result, setResult] = useState<ProveResponse | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!theoremName || !proposition || isSearching) return;
    const res = await onProve(theoremName, proposition);
    if (res) setResult(res);
  };

  const handleSelectPreset = (preset: typeof PRESET_GOALS[0]) => {
    setTheoremName(preset.name);
    setProposition(preset.prop);
  };

  return (
    <div className="flex flex-col h-full bg-slate-900 border border-slate-800 rounded-xl overflow-hidden shadow-2xl">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 bg-slate-950/80 border-b border-slate-800">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-purple-950 border border-purple-800 rounded-lg text-purple-400">
            <Sparkles className="w-5 h-5" />
          </div>
          <div>
            <h2 className="text-lg font-bold text-white tracking-wide">Interactive MCTS Theorem Prover</h2>
            <p className="text-xs text-slate-400 font-mono">
              Submit Mathlib propositions to BourbakiMuZero for game-semantic proof search & Lean 4 verification
            </p>
          </div>
        </div>
      </div>

      <div className="p-6 overflow-y-auto space-y-6 flex-1">
        {/* Preset Selectors */}
        <div>
          <label className="text-xs font-semibold text-slate-400 uppercase tracking-wider block mb-2">
            Curriculum Presets
          </label>
          <div className="flex flex-wrap gap-2">
            {PRESET_GOALS.map((preset) => (
              <button
                key={preset.name}
                type="button"
                onClick={() => handleSelectPreset(preset)}
                className={`px-3 py-1.5 rounded-lg border text-xs font-mono transition-colors ${
                  theoremName === preset.name
                    ? 'bg-purple-950 border-purple-700 text-purple-300'
                    : 'bg-slate-950/60 border-slate-800 text-slate-400 hover:text-slate-200'
                }`}
              >
                {preset.name.split('.').pop()}
              </button>
            ))}
          </div>
        </div>

        {/* Input Form */}
        <form onSubmit={handleSubmit} className="space-y-4 bg-slate-950/50 p-5 rounded-xl border border-slate-800">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="block text-xs font-semibold text-slate-300 mb-1">Theorem Name</label>
              <input
                type="text"
                value={theoremName}
                onChange={(e) => setTheoremName(e.target.value)}
                placeholder="Mathlib.Logic.And.intro"
                className="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-xs font-mono text-white focus:outline-none focus:border-purple-500"
              />
            </div>
            <div>
              <label className="block text-xs font-semibold text-slate-300 mb-1">Proposition Formula</label>
              <input
                type="text"
                value={proposition}
                onChange={(e) => setProposition(e.target.value)}
                placeholder="A -> B -> A ∧ B"
                className="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-xs font-mono text-white focus:outline-none focus:border-purple-500"
              />
            </div>
          </div>

          <div className="flex justify-end pt-2">
            <button
              type="submit"
              disabled={isSearching}
              className="flex items-center gap-2 px-5 py-2 rounded-lg bg-gradient-to-r from-blue-600 to-purple-600 hover:from-blue-500 hover:to-purple-500 text-white text-xs font-bold shadow-lg shadow-purple-900/30 transition-all disabled:opacity-50"
            >
              {isSearching ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  MCTS Tree Search In Progress...
                </>
              ) : (
                <>
                  <Play className="w-4 h-4 fill-white" />
                  Dispatch Search
                </>
              )}
            </button>
          </div>
        </form>

        {/* Results Box */}
        {result && (
          <div className="p-5 rounded-xl border border-slate-800 bg-slate-950/60 space-y-4">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <CheckCircle2 className="w-4 h-4 text-emerald-400" />
                <span className="text-sm font-bold text-white">Proof Strategy Found</span>
              </div>
              <div className="flex items-center gap-2 text-xs font-mono text-slate-400">
                <Clock className="w-3.5 h-3.5" /> {result.time_ms.toFixed(1)}ms
              </div>
            </div>

            <div className="space-y-2">
              <div className="text-xs font-semibold text-slate-300">Extracted Calculus of Inductive Constructions Term:</div>
              <div className="p-3 bg-slate-950 rounded-lg border border-slate-800 font-mono text-xs text-emerald-300">
                <pre className="whitespace-pre-wrap">{result.lean_code}</pre>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
