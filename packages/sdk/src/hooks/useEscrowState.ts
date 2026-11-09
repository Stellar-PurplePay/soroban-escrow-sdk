import { EscrowClient } from "../client";
import { EscrowRecord, Milestone, EscrowProgress } from "../types";

export interface EscrowState {
  escrow: EscrowRecord | null;
  milestones: Milestone[];
  progress: EscrowProgress | null;
  isLoading: boolean;
  error: Error | null;
  lastUpdated: number | null;
  ledgerSequence?: number;
}

export interface EscrowStateFetchResult {
  escrow: EscrowRecord;
  milestones: Milestone[];
  progress: EscrowProgress;
  ledgerSequence?: number;
}

/**
 * State manager providing reactive subscription, auto-polling, and optimistic updates
 * for Soroban escrow contract state.
 */
export class EscrowStateManager {
  private client: EscrowClient;
  private jobId: string;
  private state: EscrowState;
  private listeners: Array<(state: EscrowState) => void> = [];
  private pollIntervalId: ReturnType<typeof setInterval> | null = null;
  private isFetching = false;
  private preOptimisticSnapshot: EscrowState | null = null;

  constructor(client: EscrowClient, jobId: string) {
    this.client = client;
    this.jobId = jobId;
    this.state = {
      escrow: null,
      milestones: [],
      progress: null,
      isLoading: false,
      error: null,
      lastUpdated: null,
    };
  }

  public getState(): EscrowState {
    return { ...this.state };
  }

  public subscribe(listener: (state: EscrowState) => void): () => void {
    this.listeners.push(listener);
    listener(this.getState());
    return () => {
      this.listeners = this.listeners.filter((l) => l !== listener);
    };
  }

  private notify() {
    const current = this.getState();
    for (const listener of this.listeners) {
      listener(current);
    }
  }

  public async refresh(
    fetcher: (client: EscrowClient, jobId: string) => Promise<EscrowStateFetchResult>
  ): Promise<void> {
    if (this.isFetching) return;
    this.isFetching = true;
    this.state.isLoading = true;
    this.state.error = null;
    this.notify();

    try {
      const result = await fetcher(this.client, this.jobId);
      this.state.escrow = result.escrow;
      this.state.milestones = result.milestones;
      this.state.progress = result.progress;
      this.state.ledgerSequence = result.ledgerSequence;
      this.state.lastUpdated = Date.now();
      this.state.isLoading = false;
      this.preOptimisticSnapshot = null;
      this.notify();
    } catch (err: any) {
      this.state.error = err instanceof Error ? err : new Error(String(err));
      this.state.isLoading = false;
      this.notify();
    } finally {
      this.isFetching = false;
    }
  }

  public startPolling(
    intervalMs: number,
    fetcher: (client: EscrowClient, jobId: string) => Promise<EscrowStateFetchResult>
  ): void {
    this.stopPolling();
    this.refresh(fetcher);
    this.pollIntervalId = setInterval(() => {
      this.refresh(fetcher);
    }, Math.max(intervalMs, 500));
  }

  public stopPolling(): void {
    if (this.pollIntervalId !== null) {
      clearInterval(this.pollIntervalId);
      this.pollIntervalId = null;
    }
  }

  public isPolling(): boolean {
    return this.pollIntervalId !== null;
  }

  public applyOptimisticUpdate(update: Partial<EscrowRecord>): void {
    if (!this.state.escrow) return;
    this.preOptimisticSnapshot = { ...this.state };
    this.state.escrow = {
      ...this.state.escrow,
      ...update,
    };
    this.state.lastUpdated = Date.now();
    this.notify();
  }

  public rollbackOptimisticUpdate(): void {
    if (this.preOptimisticSnapshot) {
      this.state = { ...this.preOptimisticSnapshot };
      this.preOptimisticSnapshot = null;
      this.notify();
    }
  }

  public destroy(): void {
    this.stopPolling();
    this.listeners = [];
  }
}
