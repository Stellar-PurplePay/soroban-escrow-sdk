import { EscrowClient } from "../client";
import { Milestone, EscrowRecord } from "../types";
import { ValidationError } from "../errors";

export interface MilestoneReleaseState {
  selectedMilestoneId: number | null;
  isSubmitting: boolean;
  error: Error | null;
  txHash: string | null;
}

export interface MilestoneReleaseSummary {
  milestoneId: number;
  amount: bigint;
  percentage: number;
  newReleasedAmount: bigint;
  newRemainingAmount: bigint;
}

/**
 * Manager coordinating milestone approval and fund release transactions.
 */
export class MilestoneReleaseManager {
  private client: EscrowClient;
  private jobId: string;
  private state: MilestoneReleaseState;
  private listeners: Array<(state: MilestoneReleaseState) => void> = [];

  constructor(client: EscrowClient, jobId: string) {
    this.client = client;
    this.jobId = jobId;
    this.state = {
      selectedMilestoneId: null,
      isSubmitting: false,
      error: null,
      txHash: null,
    };
  }

  public getState(): MilestoneReleaseState {
    return { ...this.state };
  }

  public subscribe(listener: (state: MilestoneReleaseState) => void): () => void {
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

  public selectMilestone(milestoneId: number | null): void {
    this.state.selectedMilestoneId = milestoneId;
    this.state.error = null;
    this.notify();
  }

  public calculateSummary(
    milestone: Milestone,
    escrow: EscrowRecord
  ): MilestoneReleaseSummary {
    if (milestone.approved) {
      throw new ValidationError(`Milestone ${milestone.id} has already been approved and released`);
    }

    const newReleased = escrow.releasedAmount + milestone.amount;
    if (newReleased > escrow.totalAmount) {
      throw new ValidationError(
        `Milestone release would exceed escrow total amount: ${newReleased} > ${escrow.totalAmount}`
      );
    }

    const newRemaining = escrow.totalAmount - newReleased;
    return {
      milestoneId: milestone.id,
      amount: milestone.amount,
      percentage: milestone.percentage,
      newReleasedAmount: newReleased,
      newRemainingAmount: newRemaining,
    };
  }

  public async releaseMilestone(
    milestoneId: number,
    clientAddress: string,
    releaseExecutor: (jobId: string, milestoneId: number, clientAddress: string) => Promise<{ txHash: string }>
  ): Promise<string> {
    if (!milestoneId || milestoneId <= 0) {
      throw new ValidationError("Valid positive milestone ID is required");
    }
    if (!clientAddress) {
      throw new ValidationError("Client address is required for authorization");
    }

    this.state.selectedMilestoneId = milestoneId;
    this.state.isSubmitting = true;
    this.state.error = null;
    this.state.txHash = null;
    this.notify();

    try {
      const result = await releaseExecutor(this.jobId, milestoneId, clientAddress);
      this.state.txHash = result.txHash;
      this.state.isSubmitting = false;
      this.notify();
      return result.txHash;
    } catch (err: any) {
      const errorObj = err instanceof Error ? err : new Error(String(err));
      this.state.error = errorObj;
      this.state.isSubmitting = false;
      this.notify();
      throw errorObj;
    }
  }
}
