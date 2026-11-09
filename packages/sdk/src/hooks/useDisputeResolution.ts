import { EscrowClient } from "../client";
import { ValidationError } from "../errors";

export interface DisputeResolutionState {
  jobId: string;
  isSubmitting: boolean;
  error: Error | null;
  txHash: string | null;
  evidenceHash: string | null;
}

export interface SplitCalculationResult {
  clientAmount: bigint;
  freelancerAmount: bigint;
  clientPercentage: number;
  freelancerPercentage: number;
}

/**
 * Manager coordinating dispute initiation, evidence attachment, and arbitrated split settlement.
 */
export class DisputeResolutionManager {
  private client: EscrowClient;
  private jobId: string;
  private state: DisputeResolutionState;
  private listeners: Array<(state: DisputeResolutionState) => void> = [];

  constructor(client: EscrowClient, jobId: string) {
    this.client = client;
    this.jobId = jobId;
    this.state = {
      jobId,
      isSubmitting: false,
      error: null,
      txHash: null,
      evidenceHash: null,
    };
  }

  public getState(): DisputeResolutionState {
    return { ...this.state };
  }

  public subscribe(listener: (state: DisputeResolutionState) => void): () => void {
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

  public setEvidenceHash(hash: string): void {
    if (!hash || hash.trim().length === 0) {
      throw new ValidationError("Evidence hash must be non-empty");
    }
    this.state.evidenceHash = hash.trim();
    this.notify();
  }

  public calculateSplit(clientPercentage: number, remainingAmount: bigint): SplitCalculationResult {
    if (clientPercentage < 0 || clientPercentage > 100) {
      throw new ValidationError("Client percentage must be between 0 and 100");
    }
    if (remainingAmount <= 0n) {
      throw new ValidationError("Remaining amount must be positive to calculate split");
    }

    const freelancerPercentage = 100 - clientPercentage;
    const clientAmount = (remainingAmount * BigInt(clientPercentage)) / 100n;
    const freelancerAmount = remainingAmount - clientAmount;

    return {
      clientAmount,
      freelancerAmount,
      clientPercentage,
      freelancerPercentage,
    };
  }

  public async raiseDispute(
    sender: string,
    evidenceHash: string,
    executor: (jobId: string, sender: string, evidenceHash: string) => Promise<{ txHash: string }>
  ): Promise<string> {
    if (!sender) {
      throw new ValidationError("Sender address is required to raise dispute");
    }
    if (!evidenceHash) {
      throw new ValidationError("Evidence hash is required");
    }

    this.state.isSubmitting = true;
    this.state.error = null;
    this.state.evidenceHash = evidenceHash;
    this.notify();

    try {
      const result = await executor(this.jobId, sender, evidenceHash);
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

  public async resolveDispute(
    arbitrator: string,
    clientAmount: bigint,
    freelancerAmount: bigint,
    executor: (jobId: string, arbitrator: string, clientAmount: bigint, freelancerAmount: bigint) => Promise<{ txHash: string }>
  ): Promise<string> {
    if (!arbitrator) {
      throw new ValidationError("Arbitrator address is required to resolve dispute");
    }
    if (clientAmount < 0n || freelancerAmount < 0n) {
      throw new ValidationError("Dispute split amounts cannot be negative");
    }

    this.state.isSubmitting = true;
    this.state.error = null;
    this.notify();

    try {
      const result = await executor(this.jobId, arbitrator, clientAmount, freelancerAmount);
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
