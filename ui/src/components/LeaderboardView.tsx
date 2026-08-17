import React from 'react';
import { ModelRanking } from '../types';
import { Trophy, Cpu, Award } from 'lucide-react';

interface LeaderboardViewProps {
  models: ModelRanking[];
}

export const LeaderboardView: React.FC<LeaderboardViewProps> = ({ models }) => {
  return (
    <div className="flex flex-col h-full bg-slate-900 border border-slate-800 rounded-xl overflow-hidden shadow-2xl">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 bg-slate-950/80 border-b border-slate-800">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-amber-950 border border-amber-800 rounded-lg text-amber-400">
            <Trophy className="w-5 h-5" />
          </div>
          <div>
            <h2 className="text-lg font-bold text-white tracking-wide">BourbakiMuZero Champion Leaderboard</h2>
            <p className="text-xs text-slate-400 font-mono">
              Bayesian Elo Ratings &bull; Compute Simulation Equivalents (CSE) &bull; Mathlib Solve Rates
            </p>
          </div>
        </div>
      </div>

      {/* Model Cards Grid */}
      <div className="p-6 overflow-y-auto space-y-4">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
          {models.slice(0, 3).map((model, idx) => (
            <div
              key={model.name}
              className={`p-5 rounded-xl border relative overflow-hidden ${
                idx === 0
                  ? 'bg-gradient-to-b from-amber-950/30 to-slate-900 border-amber-800/60 shadow-amber-900/10 shadow-lg'
                  : 'bg-slate-950/50 border-slate-800'
              }`}
            >
              {idx === 0 && (
                <div className="absolute top-3 right-3 text-amber-400 flex items-center gap-1 text-xs font-bold">
                  <Award className="w-4 h-4" /> Champion
                </div>
              )}
              <div className="text-xs font-mono text-slate-400 mb-1">Rank #{idx + 1}</div>
              <div className="text-base font-bold text-white font-mono">{model.name}</div>
              <div className="text-2xl font-black text-amber-400 my-2 font-mono">
                {model.elo.toFixed(1)} <span className="text-xs font-normal text-slate-400">±{model.ci_95.toFixed(1)}</span>
              </div>
              <div className="flex items-center justify-between text-xs text-slate-300 pt-3 border-t border-slate-800">
                <span>Solve Rate:</span>
                <span className="font-bold text-emerald-400">
                  {((model.tier1_solve + model.tier2_solve + model.tier3_solve) / 3 * 100).toFixed(1)}%
                </span>
              </div>
            </div>
          ))}
        </div>

        {/* Detailed Table */}
        <div className="overflow-x-auto rounded-lg border border-slate-800 bg-slate-950/60">
          <table className="w-full text-left text-xs">
            <thead className="bg-slate-900/80 text-slate-400 uppercase tracking-wider font-semibold border-b border-slate-800">
              <tr>
                <th className="py-3 px-4">Model Checkpoint</th>
                <th className="py-3 px-4">Bayesian Elo (±95% CI)</th>
                <th className="py-3 px-4">Record (W-L-D)</th>
                <th className="py-3 px-4">Tier 1 Solve</th>
                <th className="py-3 px-4">Tier 2 Solve</th>
                <th className="py-3 px-4">Tier 3 Solve</th>
                <th className="py-3 px-4">Throughput (CPU)</th>
                <th className="py-3 px-4">CSE Score</th>
                <th className="py-3 px-4">Status</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800 font-mono text-slate-300">
              {models.map((model) => (
                <tr key={model.name} className="hover:bg-slate-900/40 transition-colors">
                  <td className="py-3 px-4 font-bold text-white flex items-center gap-2">
                    <Cpu className="w-3.5 h-3.5 text-blue-400" />
                    {model.name}
                  </td>
                  <td className="py-3 px-4 font-bold text-amber-400">
                    {model.elo.toFixed(1)} <span className="text-slate-500 font-normal">±{model.ci_95.toFixed(1)}</span>
                  </td>
                  <td className="py-3 px-4">
                    {model.record.wins}-{model.record.losses}-{model.record.draws} ({((model.record.wins / (model.record.wins + model.record.losses + model.record.draws || 1)) * 100).toFixed(1)}%)
                  </td>
                  <td className="py-3 px-4 text-emerald-400">{(model.tier1_solve * 100).toFixed(1)}%</td>
                  <td className="py-3 px-4 text-emerald-400">{(model.tier2_solve * 100).toFixed(1)}%</td>
                  <td className="py-3 px-4 text-emerald-400">{(model.tier3_solve * 100).toFixed(1)}%</td>
                  <td className="py-3 px-4 text-blue-400">{model.sims_per_sec.toFixed(1)} sims/s</td>
                  <td className="py-3 px-4 font-bold text-purple-400">{model.cse.toFixed(3)}x</td>
                  <td className="py-3 px-4">
                    <span className="px-2 py-0.5 rounded-full text-[10px] font-semibold bg-emerald-950 text-emerald-400 border border-emerald-800">
                      {model.status}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};
