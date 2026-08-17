import React, { useState } from 'react';
import { ProofBlockNode } from '../types';
import { GitCommit, CheckCircle2, AlertCircle, Clock, Link2, Search, Filter } from 'lucide-react';

interface ProofDagViewProps {
  nodes: ProofBlockNode[];
  edges: { source: string; target: string }[];
  onSelectBlock?: (block: ProofBlockNode) => void;
}

export const ProofDagView: React.FC<ProofDagViewProps> = ({
  nodes,
  edges,
  onSelectBlock,
}) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [filterStatus, setFilterStatus] = useState<string>('all');
  const [activeBlock, setActiveBlock] = useState<ProofBlockNode | null>(nodes[nodes.length - 1] || null);

  const filteredNodes = nodes.filter((n) => {
    const matchesSearch =
      n.theorem_name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      n.proposition.toLowerCase().includes(searchQuery.toLowerCase()) ||
      n.id.toLowerCase().includes(searchQuery.toLowerCase());

    const matchesFilter = filterStatus === 'all' || n.status === filterStatus;
    return matchesSearch && matchesFilter;
  });

  const handleSelect = (node: ProofBlockNode) => {
    setActiveBlock(node);
    if (onSelectBlock) onSelectBlock(node);
  };

  return (
    <div className="flex flex-col h-full bg-slate-900 border border-slate-800 rounded-xl overflow-hidden shadow-2xl">
      {/* Header Controls */}
      <div className="flex flex-wrap items-center justify-between gap-4 px-6 py-4 bg-slate-950/80 border-b border-slate-800">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-emerald-950 border border-emerald-800 rounded-lg text-emerald-400">
            <GitCommit className="w-5 h-5" />
          </div>
          <div>
            <h2 className="text-lg font-bold text-white tracking-wide">Decentralized Proof DAG Ledger</h2>
            <p className="text-xs text-slate-400 font-mono">
              {nodes.length} Attested Blocks &bull; {edges.length} Cryptographic Edges
            </p>
          </div>
        </div>

        {/* Search and Filters */}
        <div className="flex items-center gap-3">
          <div className="relative">
            <Search className="w-4 h-4 text-slate-400 absolute left-3 top-2.5" />
            <input
              type="text"
              placeholder="Search theorems, hashes..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9 pr-3 py-1.5 bg-slate-950 border border-slate-800 rounded-lg text-xs text-slate-200 placeholder-slate-500 focus:outline-none focus:border-blue-500 w-48 lg:w-64"
            />
          </div>

          <div className="flex items-center gap-1 bg-slate-950 border border-slate-800 p-1 rounded-lg text-xs">
            <Filter className="w-3.5 h-3.5 text-slate-400 ml-1" />
            {(['all', 'certified', 'verifying'] as const).map((status) => (
              <button
                key={status}
                onClick={() => setFilterStatus(status)}
                className={`px-2.5 py-1 rounded capitalize font-medium transition-colors ${
                  filterStatus === status
                    ? 'bg-blue-600 text-white'
                    : 'text-slate-400 hover:text-slate-200'
                }`}
              >
                {status}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* DAG Content Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-12 flex-1 overflow-hidden">
        {/* Left Column: DAG Topological Card Flow */}
        <div className="lg:col-span-7 p-6 overflow-y-auto border-r border-slate-800/80 space-y-4">
          <div className="space-y-3">
            {filteredNodes.map((node) => {
              const isSelected = activeBlock?.id === node.id;
              const isCertified = node.status === 'certified';

              return (
                <div
                  key={node.id}
                  onClick={() => handleSelect(node)}
                  className={`p-4 rounded-xl border transition-all cursor-pointer ${
                    isSelected
                      ? 'ring-2 ring-emerald-500 bg-slate-800/90 border-emerald-500/50 shadow-xl'
                      : 'bg-slate-950/60 border-slate-800/80 hover:bg-slate-800/50'
                  }`}
                >
                  <div className="flex items-start justify-between gap-2 mb-2">
                    <div>
                      <div className="flex items-center gap-2">
                        <span className="font-bold text-sm text-slate-100">{node.theorem_name}</span>
                        {isCertified ? (
                          <span className="flex items-center gap-1 text-[11px] font-semibold px-2 py-0.5 rounded-full bg-emerald-950 text-emerald-400 border border-emerald-800">
                            <CheckCircle2 className="w-3 h-3" /> Certified
                          </span>
                        ) : (
                          <span className="flex items-center gap-1 text-[11px] font-semibold px-2 py-0.5 rounded-full bg-amber-950 text-amber-400 border border-amber-800">
                            <Clock className="w-3 h-3" /> Verifying
                          </span>
                        )}
                      </div>
                      <p className="text-xs font-mono text-slate-400 mt-1">{node.proposition}</p>
                    </div>

                    <div className="text-[11px] font-mono text-slate-500 bg-slate-900 px-2 py-1 rounded border border-slate-800">
                      {node.id.slice(0, 10)}...
                    </div>
                  </div>

                  {/* Dependencies indicator */}
                  {node.parents && node.parents.length > 0 && (
                    <div className="flex items-center gap-1.5 mt-3 pt-2.5 border-t border-slate-800/60 text-[11px] font-mono text-slate-400">
                      <Link2 className="w-3.5 h-3.5 text-slate-500" />
                      <span>Dependencies ({node.parents.length}):</span>
                      {node.parents.map((p) => (
                        <span
                          key={p}
                          className="px-1.5 py-0.5 bg-slate-900 text-slate-300 rounded border border-slate-800 text-[10px]"
                        >
                          {p.slice(0, 8)}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>

        {/* Right Column: Selected Block Inspector */}
        <div className="lg:col-span-5 flex flex-col bg-slate-950/40 p-6 overflow-y-auto space-y-6">
          {activeBlock ? (
            <>
              <div>
                <div className="flex items-center justify-between mb-2">
                  <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">
                    Block Inspection
                  </span>
                  <span className="text-xs font-mono text-slate-500">
                    {new Date(activeBlock.timestamp * 1000).toLocaleTimeString()}
                  </span>
                </div>
                <h3 className="text-base font-bold text-white">{activeBlock.theorem_name}</h3>
                <p className="text-xs font-mono text-slate-400 mt-1 bg-slate-950 p-2.5 rounded border border-slate-800">
                  {activeBlock.proposition}
                </p>
              </div>

              {/* Block Hash & Integrity */}
              <div className="space-y-2">
                <div className="text-xs font-semibold text-slate-300">Content-Addressed Block ID</div>
                <div className="p-2.5 bg-slate-950 rounded border border-slate-800 text-xs font-mono text-blue-400 break-all select-all">
                  {activeBlock.id}
                </div>
              </div>

              {/* Extracted Proof Term */}
              <div className="space-y-2 flex-1 flex flex-col">
                <div className="text-xs font-semibold text-slate-300">Extracted Calculus of Inductive Constructions Term</div>
                <div className="flex-1 min-h-[140px] p-3 bg-slate-950 rounded border border-slate-800 text-xs font-mono text-emerald-300 overflow-x-auto">
                  <pre className="whitespace-pre-wrap">
                    {activeBlock.extracted_term || "fun (a : A) => fun (b : B) => And.intro a b"}
                  </pre>
                </div>
              </div>
            </>
          ) : (
            <div className="flex flex-col items-center justify-center h-full text-slate-500 text-xs">
              <AlertCircle className="w-8 h-8 mb-2 opacity-50" />
              Select a block to inspect DAG provenance
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
