import { useState, useEffect } from 'react';
import { ProofDagView } from './components/ProofDagView';
import { TelemetryFeed } from './components/TelemetryFeed';
import { UpdateNotification } from './components/UpdateNotification';
import { TargetManager } from './components/TargetManager';
import { ContributeView } from './components/ContributeView';
import { GemmaEdgePanel } from './components/GemmaEdgePanel';
import { Header, ActiveTabId } from './components/Header';
import { GuidedTour } from './components/GuidedTour';
import { initServiceWorker } from './registerServiceWorker';
import { meshClient } from './services/meshClient';
import {
  hydrateProofDag,
  saveBlocksToIndexedDB,
} from './services/telemetryClient';
import {
  ProofBlockNode,
  TelemetryEvent,
} from './types';

const INITIAL_BLOCKS: ProofBlockNode[] = [
  {
    id: "0000000000000000000000000000000000000000000000000000000000000000",
    parents: [],
    theorem_name: "Genesis",
    proposition: "True",
    extracted_term: "True.intro",
    lean_verified: true,
    timestamp: 1723900000,
    status: "certified",
  },
  {
    id: "a3f58e99bc10123d4f5e6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d",
    parents: ["0000000000000000000000000000000000000000000000000000000000000000"],
    theorem_name: "Mathlib.Logic.Identity",
    proposition: "P -> P",
    extracted_term: "fun (p : P) => p",
    lean_verified: true,
    timestamp: 1723901200,
    status: "certified",
  },
  {
    id: "b4c5d6e7f8a9b0c1d2e3f4a5b6c7da3f58e99bc10123d4f5e6a7b8c9d0e1f2a3",
    parents: ["a3f58e99bc10123d4f5e6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d"],
    theorem_name: "Mathlib.Logic.ModusPonens",
    proposition: "P -> (P -> Q) -> Q",
    extracted_term: "fun (p : P) => fun (f : P -> Q) => f p",
    lean_verified: true,
    timestamp: 1723902400,
    status: "certified",
  },
  {
    id: "c5d6e7f8a9b0c1d2e3f4a5b6c7da3f58e99bc10123d4f5e6a7b8c9d0e1f2a3b4",
    parents: ["b4c5d6e7f8a9b0c1d2e3f4a5b6c7da3f58e99bc10123d4f5e6a7b8c9d0e1f2a3"],
    theorem_name: "And.swap",
    proposition: "A ∧ B → B ∧ A",
    extracted_term: "fun (h : A ∧ B) => ⟨h.2, h.1⟩",
    lean_verified: true,
    timestamp: 1723903600,
    status: "certified",
  },
];

export function App() {
  const [activeTab, setActiveTab] = useState<ActiveTabId>('contribute');
  const [blocks, setBlocks] = useState<ProofBlockNode[]>(INITIAL_BLOCKS);
  const [telemetryEvents, setTelemetryEvents] = useState<TelemetryEvent[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const [isTourOpen, setIsTourOpen] = useState(false);

  // Check if first-time user for guided onboarding tour
  useEffect(() => {
    const tourCompleted = localStorage.getItem('bourbakimesh_tour_completed');
    if (!tourCompleted) {
      setIsTourOpen(true);
    }
  }, []);

  // Register PWA Service Worker for auto-updates
  useEffect(() => {
    initServiceWorker(() => setUpdateAvailable(true));
  }, []);

  // Fetch initial ledger status (with IndexedDB fallback)
  useEffect(() => {
    hydrateProofDag()
      .then((data) => {
        if (data.nodes && data.nodes.length > 0) {
          setBlocks(data.nodes);
        }
      })
      .catch(() => {});
  }, []);

  // Synchronize WebSocket connection and DAG events
  useEffect(() => {
    const updateConnStatus = () => {
      setIsConnected(meshClient.connectionStatus === 'connected');
    };

    updateConnStatus();
    const unsubStatus = meshClient.on('status_changed', updateConnStatus);

    const unsubTelem = meshClient.on('telemetry_updated', (telem) => {
      if (telem) {
        setTelemetryEvents((prev) => [
          {
            type: 'mesh_telemetry_tick',
            timestamp: Date.now() / 1000,
            data: telem,
          },
          ...prev,
        ].slice(0, 100));
      }
    });

    const unsubTask = meshClient.on('task_completed', (data: any) => {
      if (data && data.task) {
        const task = data.task;
        const newBlock: ProofBlockNode = {
          id: task.task_id,
          parents: task.parent ? [task.parent] : [],
          theorem_name: task.theorem_name || 'Verified CIC Target',
          proposition: task.goal_repr || task.theorem_name || 'A -> B',
          extracted_term: data.proofTerm ? JSON.stringify(data.proofTerm) : '',
          lean_verified: true,
          timestamp: Date.now() / 1000,
          status: 'certified',
        };
        setBlocks((prev) => {
          if (prev.some((b) => b.id === newBlock.id)) return prev;
          const updated = [...prev, newBlock];
          saveBlocksToIndexedDB(updated);
          return updated;
        });
      }
    });

    return () => {
      unsubStatus();
      unsubTelem();
      unsubTask();
    };
  }, []);

  return (
    <div className="flex flex-col h-screen bg-slate-950 text-slate-100">
      {/* Top Header Navbar */}
      <Header
        activeTab={activeTab}
        setActiveTab={setActiveTab}
        onOpenTour={() => setIsTourOpen(true)}
        blocksCount={blocks.length}
      />

      {/* Top-Level Swarm Objective Manager */}
      <TargetManager />

      {/* Main View Area: 4 Pillars of BourbakiMesh */}
      <main className="flex-1 p-6 overflow-y-auto">
        {/* Pillar 1: Contribute Cycles */}
        {(activeTab === 'contribute' || activeTab === 'volunteer') && <ContributeView />}

        {/* Pillar 2: Proof DAG Explorer */}
        {activeTab === 'dag' && (
          <ProofDagView
            nodes={blocks}
            edges={blocks.flatMap((b) => b.parents.map((p) => ({ source: p, target: b.id })))}
          />
        )}

        {/* Pillar 3: Model Playground */}
        {(activeTab === 'playground' || activeTab === 'gemma4') && <GemmaEdgePanel />}

        {/* Pillar 4: Flight Telemetry */}
        {activeTab === 'telemetry' && (
          <TelemetryFeed events={telemetryEvents} isConnected={isConnected} />
        )}
      </main>

      {/* Interactive Guided Tour Onboarding Walkthrough */}
      <GuidedTour
        isOpen={isTourOpen}
        onClose={() => setIsTourOpen(false)}
        onSelectTab={(tab) => setActiveTab(tab)}
      />

      {/* Auto-Updating PWA & Version Toast */}
      <UpdateNotification
        updateAvailable={updateAvailable}
        onRefresh={() => window.location.reload()}
      />
    </div>
  );
}

export default App;
