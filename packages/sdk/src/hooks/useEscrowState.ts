import { EscrowClient } from "../client";
import { EscrowRecord, Milestone, EscrowProgress } from "../types";

export interface EscrowState {
  escrow: EscrowRecord | null;
  milestones: Milestone[];
  progress: EscrowProgress | null;
  isLoading: boolean;
  error: Error | null;
}

export interface EscrowStateFetchResult {
  escrow: EscrowRecord;
  milestones: Milestone[];
  progress: EscrowProgress;
}

/**
 * State manager providing reactive subscription for Soroban escrow contract state.
 */
export class EscrowStateManager {
  private client: EscrowClient;
  private jobId: string;
  private state: EscrowState;
  private listeners: Array<(state: EscrowState) => void> = [];

  constructor(client: EscrowClient, jobId: string) {
    this.client = client;
    this.jobId = jobId;
    this.state = {
      escrow: null,
      milestones: [],
      progress: null,
      isLoading: false,
      error: null,
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
    this.state.isLoading = true;
    this.state.error = null;
    this.notify();

    try {
      const result = await fetcher(this.client, this.jobId);
      this.state.escrow = result.escrow;
      this.state.milestones = result.milestones;
      this.state.progress = result.progress;
      this.state.isLoading = false;
      this.notify();
    } catch (err: any) {
      this.state.error = err instanceof Error ? err : new Error(String(err));
      this.state.isLoading = false;
      this.notify();
    }
  }
}
