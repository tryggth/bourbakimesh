/**
 * BourbakiMesh Flight Recorder Telemetry Service (EventTracer).
 *
 * Implements structured, non-blocking trace recording for neural-symbolic
 * proof search replay debugging, state diff inspection, and JSONL session export.
 */

export type TraceEventType =
  | 'SESSION_START'
  | 'ACTOR_EXPAND'
  | 'CRITIC_SCORE'
  | 'KERNEL_TRANSITION'
  | 'NODE_PRUNED'
  | 'PROOF_CLOSED';

export interface StateDiff {
  addedHyp?: { id: string; expr: any };
  removedHyp?: string;
  currentHyps: Record<string, any>;
  target: any;
}

export interface TraceEvent {
  id: string;
  type: TraceEventType;
  timestampUs: number; // Monotonic microseconds
  isoTimestamp: string;
  nodeId?: string;
  parentId?: string | null;
  stepAst?: any;
  rawAstJson?: string;
  thinkingTrace?: string;
  promptTokens?: number;
  tokensPerSec?: number;
  genrmScore?: number;
  kernelLatencyUs?: number;
  stateDiff?: StateDiff;
  status?: string;
  error?: string;
}

type TraceListener = (event: TraceEvent) => void;

class EventTracer {
  private traceHistory: TraceEvent[] = [];
  private listeners: TraceListener[] = [];
  private sessionStartTime: number = performance.now();

  /**
   * Subscribe to new flight telemetry events.
   */
  public subscribe(cb: TraceListener): () => void {
    this.listeners.push(cb);
    return () => {
      this.listeners = this.listeners.filter((l) => l !== cb);
    };
  }

  /**
   * Append a structured trace event.
   */
  public recordEvent(event: Omit<TraceEvent, 'id' | 'timestampUs' | 'isoTimestamp'>): TraceEvent {
    const nowMs = performance.now();
    const timestampUs = Math.round((nowMs - this.sessionStartTime) * 1000);
    const fullEvent: TraceEvent = {
      id: `evt-${Date.now().toString(36)}-${Math.random().toString(36).substring(2, 6)}`,
      timestampUs,
      isoTimestamp: new Date().toISOString(),
      ...event,
    };

    this.traceHistory.push(fullEvent);

    for (const listener of this.listeners) {
      try {
        listener(fullEvent);
      } catch (err) {
        console.error('[EventTracer] Listener error:', err);
      }
    }

    return fullEvent;
  }

  /**
   * Retrieve all recorded events.
   */
  public getTraceHistory(): TraceEvent[] {
    return [...this.traceHistory];
  }

  /**
   * Export execution trace history as a standard JSONL document.
   */
  public exportTraceJsonl(): string {
    return this.traceHistory.map((evt) => JSON.stringify(evt)).join('\n');
  }

  /**
   * Clear recorded telemetry.
   */
  public clear(): void {
    this.traceHistory = [];
    this.sessionStartTime = performance.now();
  }
}

export const eventTracer = new EventTracer();
