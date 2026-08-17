import React from 'react';
import { TelemetryEvent } from '../types';
import { Activity, Radio, GitPullRequest, CheckCircle2 } from 'lucide-react';

interface TelemetryFeedProps {
  events: TelemetryEvent[];
  isConnected: boolean;
}

export const TelemetryFeed: React.FC<TelemetryFeedProps> = ({ events, isConnected }) => {
  return (
    <div className="flex flex-col h-full bg-slate-900 border border-slate-800 rounded-xl overflow-hidden shadow-2xl">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 bg-slate-950/80 border-b border-slate-800">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-blue-950 border border-blue-800 rounded-lg text-blue-400">
            <Activity className="w-5 h-5" />
          </div>
          <div>
            <h2 className="text-lg font-bold text-white tracking-wide">Live Swarm Telemetry</h2>
            <p className="text-xs text-slate-400 font-mono">
              WebSocket stream: /ws/telemetry &bull; Real-time MCTS rollouts & block attestations
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <span
            className={`w-2.5 h-2.5 rounded-full ${
              isConnected ? 'bg-emerald-500 animate-pulse' : 'bg-rose-500'
            }`}
          />
          <span className="text-xs font-mono text-slate-300">
            {isConnected ? 'LIVE STREAMING' : 'DISCONNECTED'}
          </span>
        </div>
      </div>

      {/* Events List */}
      <div className="flex-1 p-6 overflow-y-auto space-y-3 font-mono text-xs">
        {events.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-slate-500">
            <Radio className="w-8 h-8 mb-2 opacity-40 animate-pulse" />
            Listening for swarm telemetry events...
          </div>
        ) : (
          events.map((ev, idx) => (
            <div
              key={idx}
              className="p-3 bg-slate-950/80 border border-slate-800/80 rounded-lg flex items-start gap-3 hover:border-slate-700 transition-colors"
            >
              <div className="mt-0.5">{getEventIcon(ev.type)}</div>
              <div className="flex-1 min-w-0">
                <div className="flex items-center justify-between text-slate-400 text-[11px] mb-1">
                  <span className="font-semibold text-slate-300 uppercase">{ev.type}</span>
                  <span>{new Date(ev.timestamp * 1000).toLocaleTimeString()}</span>
                </div>
                <div className="text-slate-300 whitespace-pre-wrap break-all text-[11px]">
                  {JSON.stringify(ev.data, null, 2)}
                </div>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
};

function getEventIcon(type: string) {
  switch (type) {
    case 'proof_attested':
      return <CheckCircle2 className="w-4 h-4 text-emerald-400" />;
    case 'mcts_step':
      return <Activity className="w-4 h-4 text-blue-400" />;
    case 'peer_connected':
      return <GitPullRequest className="w-4 h-4 text-purple-400" />;
    default:
      return <Radio className="w-4 h-4 text-slate-400" />;
  }
}
