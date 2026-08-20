import { ProofBlockNode, ProofDagData, DaemonStatus, TelemetryEvent } from '../types';
import { meshClient } from './meshClient';

const DB_NAME = 'BourbakiMeshDB';
const DB_VERSION = 1;
const STORE_NAME = 'proof_blocks';

/**
 * Open or initialize the local IndexedDB proof DAG storage.
 */
function openIndexedDB(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    if (typeof indexedDB === 'undefined') {
      return reject(new Error('IndexedDB not supported'));
    }
    const request = indexedDB.open(DB_NAME, DB_VERSION);
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        db.createObjectStore(STORE_NAME, { keyPath: 'id' });
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

/**
 * Persist block nodes to local IndexedDB.
 */
export async function saveBlocksToIndexedDB(blocks: ProofBlockNode[]): Promise<void> {
  try {
    const db = await openIndexedDB();
    const tx = db.transaction(STORE_NAME, 'readwrite');
    const store = tx.objectStore(STORE_NAME);
    for (const block of blocks) {
      store.put(block);
    }
  } catch (err) {
    console.warn('[IndexedDB] Failed to cache blocks locally:', err);
  }
}

/**
 * Retrieve cached block nodes from local IndexedDB.
 */
export async function getBlocksFromIndexedDB(): Promise<ProofBlockNode[]> {
  try {
    const db = await openIndexedDB();
    return new Promise((resolve) => {
      const tx = db.transaction(STORE_NAME, 'readonly');
      const store = tx.objectStore(STORE_NAME);
      const req = store.getAll();
      req.onsuccess = () => resolve(req.result || []);
      req.onerror = () => resolve([]);
    });
  } catch {
    return [];
  }
}

/**
 * Hydrate proof DAG data from coordinator JSON-RPC or fallback to local IndexedDB.
 */
export async function hydrateProofDag(): Promise<ProofDagData> {
  // 1. Try WebSocket JSON-RPC to Coordinator
  try {
    const dag = await meshClient.getDag();
    if (dag && dag.nodes && Object.keys(dag.nodes).length > 0) {
      const nodes: ProofBlockNode[] = Object.values(dag.nodes).map((n: any) => ({
        id: n.id,
        parents: n.parent ? [n.parent] : [],
        theorem_name: n.goal_repr || n.id,
        proposition: n.goal_repr || 'CIC Target',
        extracted_term: n.cic_proof_term ? JSON.stringify(n.cic_proof_term) : '',
        lean_verified: n.status === 'Proven',
        timestamp: Date.now() / 1000,
        status: (n.status === 'Proven' ? 'certified' : 'verifying') as 'certified' | 'verifying',
      }));
      const edges: { source: string; target: string }[] = [];
      for (const node of nodes) {
        for (const parent of node.parents) {
          edges.push({ source: parent, target: node.id });
        }
      }
      await saveBlocksToIndexedDB(nodes);
      return { nodes, edges };
    }
  } catch {
    // Coordinator offline / not connected yet
  }

  // 2. Try IndexedDB Cached Blocks
  const cachedBlocks = await getBlocksFromIndexedDB();
  if (cachedBlocks.length > 0) {
    const edges: { source: string; target: string }[] = [];
    for (const block of cachedBlocks) {
      for (const parent of block.parents) {
        edges.push({ source: parent, target: block.id });
      }
    }
    return { nodes: cachedBlocks, edges };
  }

  // 3. Default minimal genesis
  return {
    nodes: [
      {
        id: '0000000000000000000000000000000000000000000000000000000000000000',
        parents: [],
        theorem_name: 'Genesis',
        proposition: 'True',
        extracted_term: 'True.intro',
        lean_verified: true,
        timestamp: Date.now() / 1000,
        status: 'certified',
      },
    ],
    edges: [],
  };
}

/**
 * Determine the active telemetry WebSocket URL with fallback gateway resolution.
 */
export function getTelemetryWebSocketUrl(): string {
  // Check custom user-configured gateway in localStorage
  if (typeof localStorage !== 'undefined') {
    const customGateway = localStorage.getItem('custom_gateway');
    if (customGateway) return customGateway;
  }

  // Determine based on current browser window location
  if (typeof window !== 'undefined' && window.location) {
    // If running on GitHub Pages or external domain without custom gateway, use public relay
    if (window.location.hostname.endsWith('github.io')) {
      return 'wss://relay.bourbakimesh.org/ws/telemetry';
    }

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    return `${protocol}//${host}/ws/telemetry`;
  }

  return 'ws://127.0.0.1:8000/ws/telemetry';
}

/**
 * Fetch Coordinator status with offline fallback.
 */
export async function fetchDaemonStatus(): Promise<DaemonStatus> {
  try {
    const telem = await meshClient.getCoordinatorTelemetry();
    if (telem) {
      return {
        status: 'online',
        active_model: 'gemma-4-2b-it-q4f16-webgpu',
        peer_count: telem.active_workers || 1,
        total_blocks: telem.total_nodes || 0,
        certified_blocks: telem.proven_nodes || 0,
        uptime_seconds: 3600,
        cse_score: 1.58,
        hardware: {
          cpu_cores: typeof navigator !== 'undefined' ? navigator.hardwareConcurrency || 4 : 4,
          torch_device: 'webgpu',
          memory_gb: 16,
        },
      };
    }
  } catch {
    // Standalone / offline mode
  }

  return {
    status: 'standalone-pwa',
    active_model: 'gemma-4-2b-it-q4f16-webgpu',
    peer_count: 1,
    total_blocks: 4,
    certified_blocks: 4,
    uptime_seconds: 3600,
    cse_score: 1.58,
    hardware: {
      cpu_cores: typeof navigator !== 'undefined' ? navigator.hardwareConcurrency || 4 : 4,
      torch_device: 'webgpu',
      memory_gb: 16,
    },
  };
}

export type { TelemetryEvent };

