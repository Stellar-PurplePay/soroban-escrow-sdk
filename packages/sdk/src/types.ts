export enum EscrowStatus {
  Created = "Created",
  InProgress = "InProgress",
  Completed = "Completed",
  Refunded = "Refunded",
  Disputed = "Disputed",
}

export interface Milestone {
  id: number;
  description: string;
  percentage: number;
  amount: bigint;
  approved: boolean;
}

export interface MilestoneInput {
  description: string;
  percentage: number;
}

export interface EscrowRecord {
  jobId: string;
  client: string;
  freelancer: string;
  token: string;
  totalAmount: bigint;
  releasedAmount: bigint;
  status: EscrowStatus;
  createdAt: number;
  timeoutTimestamp: number;
  arbitrator: string;
}

export interface CreateEscrowOptions {
  jobId: string;
  client: string;
  freelancer: string;
  tokenAddress: string;
  amount: bigint;
  timeoutTimestamp: number;
  arbitratorAddress: string;
  milestones?: MilestoneInput[];
}

export interface EscrowConfig {
  rpcUrl: string;
  networkPassphrase: string;
  contractId: string;
}

export interface SimulationResult<T> {
  success: boolean;
  returnValue?: T;
  feeStroops: number;
  rawSimulation: any;
}

export interface ArbitratorVote {
  arbitrator: string;
  clientAward: bigint;
  freelancerAward: bigint;
}

export interface EscrowProgress {
  totalAmount: bigint;
  releasedAmount: bigint;
  percentageReleased: number;
  status: EscrowStatus;
}

export interface ContractHealthReport {
  contractId: string;
  isResponsive: boolean;
  isPaused: boolean;
  rpcLatencyMs: number;
  latestLedgerSequence?: number;
  error?: string;
}

export interface PaymentReceiptEvent {
  receiptId: number;
  jobId: string;
  milestoneId: number;
  recipient: string;
  amount: bigint;
  fee: bigint;
}
