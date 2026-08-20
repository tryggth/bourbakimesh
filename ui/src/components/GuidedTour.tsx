import React, { useState, useEffect } from 'react';
import {
  X,
  ChevronRight,
  ChevronLeft,
  CheckCircle2,
  Cpu,
  Sparkles,
  ShieldCheck,
  GitCommit,
  Brain,
} from 'lucide-react';

export interface GuidedTourProps {
  isOpen: boolean;
  onClose: () => void;
  onSelectTab?: (tab: 'contribute' | 'dag' | 'playground' | 'telemetry') => void;
}

interface TourStep {
  id: string;
  stepNumber: number;
  title: string;
  subtitle: string;
  badge: string;
  badgeColor: string;
  description: string;
  bulletPoints: string[];
  tabTarget?: 'contribute' | 'dag' | 'playground' | 'telemetry';
  icon: React.ReactNode;
}

const TOUR_STEPS: TourStep[] = [
  {
    id: 'swarm-header',
    stepNumber: 1,
    title: 'Decentralized Swarm & WebGPU Runtime',
    subtitle: 'Phase 5 Edge Telemetry & Real-Time Monitoring',
    badge: 'Gemma 4 Edge (2B-IT W4A16)',
    badgeColor: 'teal',
    description:
      'BourbakiMesh transforms connected web browsers into a cooperative proving swarm. The top navigation bar displays live telemetry: connected edge worker count, verified Mathlib theorem progress, and active WebGPU VRAM allocation (~1.85 GB).',
    bulletPoints: [
      'Live coordinator WebSocket sync & real-time connected edge worker count',
      'Real-time VRAM buffer allocation with shader-f16 hardware acceleration',
      'Instant visual tracking of verified targets across open Mathlib conjectures',
    ],
    icon: <Cpu className="w-6 h-6 text-teal-400" />,
  },
  {
    id: 'autonomous-worker',
    stepNumber: 2,
    title: 'Autonomous Edge Volunteer Worker',
    subtitle: 'Distributed Task Leasing & Formal Proof Synthesis',
    badge: 'Contribute Cycles Pillar',
    badgeColor: 'blue',
    tabTarget: 'contribute',
    description:
      'Contribute computational cycles directly to formal mathematics. When the Autonomous Solver is activated, your browser leases unproven sub-goals from the coordinator, synthesizes formal constructive or classical proof terms, and submits certified flight attestations.',
    bulletPoints: [
      'Zero-install browser volunteer computing running purely in client Web Workers',
      'Automated goal leasing, timeout recovery, and continuous proof dispatch',
      'Ability to inject open Mathlib conjectures to distribute exploration across peers',
    ],
    icon: <Sparkles className="w-6 h-6 text-blue-400" />,
  },
  {
    id: 'wasm-kernel',
    stepNumber: 3,
    title: 'WASM Kernel Pre-Verification',
    subtitle: 'Calculus of Inductive Constructions (CIC) Type Checker',
    badge: 'Zero-Sorry Guarantee',
    badgeColor: 'emerald',
    tabTarget: 'contribute',
    description:
      'Before any proof term is submitted to the coordinator ledger, it is verified inside a high-speed Rust-compiled WebAssembly CIC kernel. Proof terms must be constructive or explicitly leverage sound classical axioms (e.g. Classical.em), guaranteeing zero hallucinated proofs.',
    bulletPoints: [
      'Deterministic sub-millisecond local kernel validation (< 0.1ms latency)',
      'Strict Calculus of Inductive Constructions with inductive recursor reduction',
      'Immediate rejection of malformed de Bruijn indices and unverified axioms',
    ],
    icon: <ShieldCheck className="w-6 h-6 text-emerald-400" />,
  },
  {
    id: 'proof-dag',
    stepNumber: 4,
    title: 'Proof DAG & Goal Decomposition',
    subtitle: 'Dynamic Exploration Frontier of Mathematical Knowledge',
    badge: 'Proof DAG Explorer Pillar',
    badgeColor: 'indigo',
    tabTarget: 'dag',
    description:
      'Inspect the global Directed Acyclic Graph (DAG) of certified mathematical blocks. Visualizes atomic lemmas, dependency linkages, inductive hypothesis decomposition, and certified Mathlib propositions.',
    bulletPoints: [
      'Interactive block visualizer with genesis-to-leaf lineage mapping',
      'Real-time attestation stream from distributed edge worker nodes',
      'Inspect extracted Lean 4 proof terms and cryptographic verification certificates',
    ],
    icon: <GitCommit className="w-6 h-6 text-indigo-400" />,
  },
  {
    id: 'neuro-symbolic',
    stepNumber: 5,
    title: 'Neuro-Symbolic Synthesis Architecture',
    subtitle: 'Tier 1 Symbolic Fast Path + Tier 2 Neural Search Fallback',
    badge: 'Dual-Tier Search Architecture',
    badgeColor: 'purple',
    tabTarget: 'playground',
    description:
      'BourbakiMesh employs a two-tier hybrid proving strategy. Purely constructive goals are solved instantaneously via Tier 1 symbolic backward-chaining. Complex or classical targets escalate to Tier 2 Best-First Search using Gemma 4 (Actor) and GenRM logprob scoring (Critic).',
    bulletPoints: [
      'Tier 1: Instantaneous de Bruijn constructive term synthesis (< 1ms)',
      'Tier 2: Actor-Critic Best-First Search with reasoning traces inside <think> tags',
      'Rich flight telemetry logging search tree depth, nodes explored, and escalation triggers',
    ],
    icon: <Brain className="w-6 h-6 text-purple-400" />,
  },
];

export const GuidedTour: React.FC<GuidedTourProps> = ({
  isOpen,
  onClose,
  onSelectTab,
}) => {
  const [currentStepIndex, setCurrentStepIndex] = useState<number>(0);

  const step = TOUR_STEPS[currentStepIndex];

  // Auto-switch to relevant tab when step changes
  useEffect(() => {
    if (isOpen && step.tabTarget && onSelectTab) {
      onSelectTab(step.tabTarget);
    }
  }, [isOpen, currentStepIndex, step.tabTarget, onSelectTab]);

  // Keyboard navigation
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        handleDismiss();
      } else if (e.key === 'ArrowRight') {
        handleNext();
      } else if (e.key === 'ArrowLeft') {
        handlePrevious();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, currentStepIndex]);

  if (!isOpen) return null;

  const handleDismiss = () => {
    localStorage.setItem('bourbakimesh_tour_completed', 'true');
    onClose();
  };

  const handleNext = () => {
    if (currentStepIndex < TOUR_STEPS.length - 1) {
      setCurrentStepIndex(currentStepIndex + 1);
    } else {
      handleDismiss();
    }
  };

  const handlePrevious = () => {
    if (currentStepIndex > 0) {
      setCurrentStepIndex(currentStepIndex - 1);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-md animate-in fade-in duration-200">
      <div className="relative w-full max-w-2xl bg-slate-900 border border-slate-700/80 rounded-2xl shadow-2xl shadow-purple-950/50 overflow-hidden flex flex-col">
        {/* Top Header with Step Tracker */}
        <div className="flex items-center justify-between px-6 py-4 bg-slate-950/90 border-b border-slate-800">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-xl bg-slate-900 border border-slate-700/60 shadow-md">
              {step.icon}
            </div>
            <div>
              <div className="flex items-center gap-2">
                <span className="text-[11px] font-mono font-bold uppercase tracking-wider text-slate-400">
                  BourbakiMesh Guided Tour
                </span>
                <span className="px-2 py-0.5 text-[10px] font-mono font-bold bg-indigo-950 text-indigo-300 border border-indigo-700/50 rounded-full">
                  Step {step.stepNumber} of {TOUR_STEPS.length}
                </span>
              </div>
              <div className="text-xs text-slate-400 font-mono mt-0.5">{step.subtitle}</div>
            </div>
          </div>

          <button
            type="button"
            onClick={handleDismiss}
            className="p-1.5 text-slate-400 hover:text-white hover:bg-slate-800 rounded-lg transition-colors"
            title="Close Tour (Esc)"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Step Progress Indicators */}
        <div className="flex items-center gap-1.5 px-6 pt-4 pb-2 bg-slate-900">
          {TOUR_STEPS.map((s, idx) => (
            <button
              key={s.id}
              type="button"
              onClick={() => setCurrentStepIndex(idx)}
              className={`flex-1 h-1.5 rounded-full transition-all duration-300 ${
                idx === currentStepIndex
                  ? 'bg-gradient-to-r from-teal-400 via-blue-500 to-indigo-500 shadow-sm shadow-indigo-500/50'
                  : idx < currentStepIndex
                  ? 'bg-emerald-500/80'
                  : 'bg-slate-800 hover:bg-slate-700'
              }`}
              title={`Jump to Step ${s.stepNumber}: ${s.title}`}
            />
          ))}
        </div>

        {/* Modal Main Body */}
        <div className="px-6 py-5 space-y-4">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h3 className="text-lg font-bold text-white tracking-wide">{step.title}</h3>
            <span
              className={`px-2.5 py-0.5 text-[11px] font-mono font-semibold rounded-full border ${
                step.badgeColor === 'teal'
                  ? 'bg-teal-950/80 text-teal-300 border-teal-700/60'
                  : step.badgeColor === 'blue'
                  ? 'bg-blue-950/80 text-blue-300 border-blue-700/60'
                  : step.badgeColor === 'emerald'
                  ? 'bg-emerald-950/80 text-emerald-300 border-emerald-700/60'
                  : step.badgeColor === 'indigo'
                  ? 'bg-indigo-950/80 text-indigo-300 border-indigo-700/60'
                  : 'bg-purple-950/80 text-purple-300 border-purple-700/60'
              }`}
            >
              {step.badge}
            </span>
          </div>

          <p className="text-sm text-slate-300 leading-relaxed font-sans">{step.description}</p>

          <div className="bg-slate-950/60 p-3.5 rounded-xl border border-slate-800/80 space-y-2">
            <div className="text-[11px] font-mono font-bold text-slate-400 uppercase tracking-wider">
              Key Capabilities:
            </div>
            <ul className="space-y-1.5">
              {step.bulletPoints.map((bullet, idx) => (
                <li key={idx} className="flex items-start gap-2 text-xs font-mono text-slate-300">
                  <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400 flex-shrink-0 mt-0.5" />
                  <span>{bullet}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>

        {/* Bottom Actions Bar */}
        <div className="flex items-center justify-between px-6 py-4 bg-slate-950 border-t border-slate-800">
          <button
            type="button"
            onClick={handleDismiss}
            className="text-xs font-mono text-slate-400 hover:text-slate-200 transition-colors px-2 py-1"
          >
            Skip Tour
          </button>

          <div className="flex items-center gap-2">
            {currentStepIndex > 0 && (
              <button
                type="button"
                onClick={handlePrevious}
                className="flex items-center gap-1 px-3.5 py-2 text-xs font-mono font-semibold bg-slate-800 hover:bg-slate-700 text-slate-200 rounded-lg border border-slate-700 transition-colors"
              >
                <ChevronLeft className="w-3.5 h-3.5" />
                Previous
              </button>
            )}

            <button
              type="button"
              onClick={handleNext}
              className="flex items-center gap-1.5 px-4 py-2 text-xs font-mono font-bold bg-gradient-to-r from-blue-600 to-indigo-600 hover:from-blue-500 hover:to-indigo-500 text-white rounded-lg shadow-lg shadow-blue-900/30 transition-all"
            >
              {currentStepIndex === TOUR_STEPS.length - 1 ? (
                <>
                  <span>Finish Tour</span>
                  <CheckCircle2 className="w-3.5 h-3.5" />
                </>
              ) : (
                <>
                  <span>Next Step</span>
                  <ChevronRight className="w-3.5 h-3.5" />
                </>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
