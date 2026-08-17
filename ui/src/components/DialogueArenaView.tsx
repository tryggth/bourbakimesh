import React, { useState } from 'react';
import { DialogueMove } from '../types';
import { ShieldCheck, ShieldAlert, ArrowRight, CornerDownRight, Eye, Code, Zap } from 'lucide-react';

interface DialogueArenaViewProps {
  moves: DialogueMove[];
  leanCode?: string;
  isVerified?: boolean;
  theoremName?: string;
  proposition?: string;
}

export const DialogueArenaView: React.FC<DialogueArenaViewProps> = ({
  moves,
  leanCode,
  isVerified = true,
  theoremName = "Mathlib.Logic.And.intro",
  proposition = "A -> B -> A ∧ B",
}) => {
  const [selectedMoveId, setSelectedMoveId] = useState<number | null>(null);

  const selectedMove = moves.find((m) => m.id === selectedMoveId) || moves[moves.length - 1];

  return (
    <div className="flex flex-col h-full bg-slate-900 border border-slate-800 rounded-xl overflow-hidden shadow-2xl">
      {/* Header Bar */}
      <div className="flex items-center justify-between px-6 py-4 bg-slate-950/80 border-b border-slate-800">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-blue-950 border border-blue-800 rounded-lg text-blue-400">
            <Zap className="w-5 h-5" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h2 className="text-lg font-bold text-white tracking-wide">{theoremName}</h2>
              {isVerified ? (
                <span className="flex items-center gap-1 text-xs font-semibold px-2 py-0.5 rounded-full bg-emerald-950 text-emerald-400 border border-emerald-800">
                  <ShieldCheck className="w-3.5 h-3.5" /> Lean 4 Certified
                </span>
              ) : (
                <span className="flex items-center gap-1 text-xs font-semibold px-2 py-0.5 rounded-full bg-amber-950 text-amber-400 border border-amber-800">
                  <ShieldAlert className="w-3.5 h-3.5" /> Unverified
                </span>
              )}
            </div>
            <p className="text-xs font-mono text-slate-400 mt-0.5">{proposition}</p>
          </div>
        </div>

        <div className="flex items-center gap-4 text-xs font-mono">
          <div className="flex items-center gap-1.5 text-blue-400 bg-blue-950/50 px-2.5 py-1 rounded-md border border-blue-900">
            <span className="w-2 h-2 rounded-full bg-blue-500 animate-pulse"></span>
            Proponent (P)
          </div>
          <div className="flex items-center gap-1.5 text-orange-400 bg-orange-950/50 px-2.5 py-1 rounded-md border border-orange-900">
            <span className="w-2 h-2 rounded-full bg-orange-500"></span>
            Opponent (O)
          </div>
        </div>
      </div>

      {/* Main Split Body: Left Play Trace, Right View Inspector & Lean Code */}
      <div className="grid grid-cols-1 lg:grid-cols-12 flex-1 overflow-hidden">
        {/* Left Column: Move Sequence Dialogue Tree */}
        <div className="lg:col-span-7 p-6 overflow-y-auto border-r border-slate-800/80 space-y-4">
          <div className="text-xs font-semibold uppercase tracking-wider text-slate-400 mb-2 flex items-center justify-between">
            <span>Lorenzen / Hyland-Ong Dialogue Trace ({moves.length} Moves)</span>
            <span className="text-[10px] text-slate-500">Click move to inspect P-view</span>
          </div>

          <div className="relative space-y-3">
            {moves.map((move, index) => {
              const isProponent = move.player === 'P';
              const isSelected = selectedMove?.id === move.id;

              return (
                <div
                  key={move.id}
                  onClick={() => setSelectedMoveId(move.id)}
                  className={`relative flex items-start gap-3 p-3.5 rounded-lg border transition-all cursor-pointer ${
                    isSelected
                      ? 'ring-2 ring-blue-500 bg-slate-800/90 border-blue-500/50 shadow-lg'
                      : isProponent
                      ? 'bg-blue-950/20 border-blue-900/40 hover:bg-blue-950/40'
                      : 'bg-orange-950/20 border-orange-900/40 hover:bg-orange-950/40'
                  }`}
                >
                  {/* Step Index & Player Avatar */}
                  <div
                    className={`flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center font-bold text-xs border ${
                      isProponent
                        ? 'bg-blue-600 text-white border-blue-400 shadow-blue-500/20 shadow-md'
                        : 'bg-orange-600 text-white border-orange-400 shadow-orange-500/20 shadow-md'
                    }`}
                  >
                    {move.player}
                  </div>

                  {/* Move Details */}
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center justify-between text-xs mb-1">
                      <span className="font-semibold text-slate-200">
                        Move #{move.id} &bull; {move.kind}
                      </span>
                      {move.justification_id !== null && (
                        <span className="flex items-center gap-1 text-[11px] font-mono text-slate-400 bg-slate-950 px-2 py-0.5 rounded border border-slate-800">
                          <CornerDownRight className="w-3 h-3 text-slate-500" />
                          enables #{move.justification_id}
                        </span>
                      )}
                    </div>

                    {/* Move Payload Content */}
                    <div className="text-xs font-mono text-slate-300 bg-slate-950/60 p-2 rounded border border-slate-800/80">
                      {formatPayload(move)}
                    </div>
                  </div>

                  {/* Justification Connection Arc Indicator */}
                  {index > 0 && move.justification_id !== null && (
                    <div className="text-[10px] text-slate-500 font-mono self-center">
                      <ArrowRight className="w-3.5 h-3.5" />
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>

        {/* Right Column: Scoping View Inspector and Extracted Lean Term */}
        <div className="lg:col-span-5 flex flex-col bg-slate-950/40 p-6 overflow-y-auto space-y-6">
          {/* Scoping Stack Inspector */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <h3 className="text-xs font-semibold uppercase tracking-wider text-slate-400 flex items-center gap-2">
                <Eye className="w-4 h-4 text-blue-400" />
                Scoping Stack Inspector (Move #{selectedMove?.id ?? 0})
              </h3>
            </div>

            <div className="grid grid-cols-2 gap-3">
              {/* P-View */}
              <div className="p-3 bg-blue-950/30 border border-blue-900/50 rounded-lg">
                <div className="text-[11px] font-semibold text-blue-400 mb-1.5">Active P-View ($\ulcorner s \urcorner$)</div>
                <div className="flex flex-wrap gap-1.5">
                  {selectedMove?.p_view && selectedMove.p_view.length > 0 ? (
                    selectedMove.p_view.map((step) => (
                      <span
                        key={step}
                        className="px-2 py-0.5 text-xs font-mono bg-blue-900/60 text-blue-200 rounded border border-blue-700"
                      >
                        #{step}
                      </span>
                    ))
                  ) : (
                    <span className="text-xs text-slate-500 font-mono">Empty</span>
                  )}
                </div>
              </div>

              {/* O-View */}
              <div className="p-3 bg-orange-950/30 border border-orange-900/50 rounded-lg">
                <div className="text-[11px] font-semibold text-orange-400 mb-1.5">Active O-View ($\llcorner s \lrcorner$)</div>
                <div className="flex flex-wrap gap-1.5">
                  {selectedMove?.o_view && selectedMove.o_view.length > 0 ? (
                    selectedMove.o_view.map((step) => (
                      <span
                        key={step}
                        className="px-2 py-0.5 text-xs font-mono bg-orange-900/60 text-orange-200 rounded border border-orange-700"
                      >
                        #{step}
                      </span>
                    ))
                  ) : (
                    <span className="text-xs text-slate-500 font-mono">Empty</span>
                  )}
                </div>
              </div>
            </div>
          </div>

          {/* Extracted Proof Term */}
          <div className="space-y-3 flex-1 flex flex-col">
            <h3 className="text-xs font-semibold uppercase tracking-wider text-slate-400 flex items-center gap-2">
              <Code className="w-4 h-4 text-emerald-400" />
              Constructive Calculus of Inductive Constructions (CIC) Term
            </h3>

            <div className="relative flex-1 min-h-[160px] bg-slate-950 border border-slate-800 rounded-lg p-4 font-mono text-xs text-emerald-300 overflow-x-auto shadow-inner">
              <pre className="whitespace-pre-wrap leading-relaxed">
                {leanCode || "fun (hyp_0 : A_0) => fun (hyp_1 : A_1) => And.intro hyp_0 hyp_1"}
              </pre>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

function formatPayload(move: DialogueMove): string {
  const p = move.payload;
  if (!p) return "Action";
  switch (p.type) {
    case "AttackHypothesis":
      return `?attack_hyp(hyp_${p.hyp_id})`;
    case "AxiomDischarge":
      return `!discharge_with(hyp_${p.premise_id})`;
    case "ProvideWitness":
      return `!witness(${p.term_repr || 't'})`;
    case "AttackConjunction":
      return `?attack_and(${p.branch || 'left'})`;
    case "RootGoal":
      return `!claim_root("${p.term_repr || 'Goal'}")`;
    default:
      return `${p.type}: ${JSON.stringify(p)}`;
  }
}
