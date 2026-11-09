import {
  CreateEscrowOptions,
  EscrowConfig,
  EscrowRecord,
  EscrowStatus,
  Milestone,
  ContractHealthReport,
} from "./types";
import { validateMilestones, isValidStellarAddress } from "./utils";
import { ValidationError } from "./errors";
import { parseResourceEstimate, ResourceCost, ResourceBudgetLimits } from "./estimator";
import { signWithFreighter, FreighterSignOptions, FreighterApi } from "./freighter";
import { withRpcRetry, RpcRetryOptions } from "./rpcRetry";
import {
  partitionPayments,
  PaymentItem,
  BatchPaymentSummary,
  PartitionOptions,
} from "./batchPayment";
import { toStroops } from "./utils/stroopPrecision";

export class EscrowClient {
  public readonly rpcUrl: string;
  public readonly networkPassphrase: string;
  public readonly contractId: string;

  constructor(config: EscrowConfig) {
    if (!config.rpcUrl) {
      throw new ValidationError("rpcUrl is required");
    }
    if (!config.networkPassphrase) {
      throw new ValidationError("networkPassphrase is required");
    }
    if (!config.contractId) {
      throw new ValidationError("contractId is required");
    }

    this.rpcUrl = config.rpcUrl;
    this.networkPassphrase = config.networkPassphrase;
    this.contractId = config.contractId;
  }

  /**
   * Validate parameters and build operation payload for creating an escrow.
   */
  public prepareCreateEscrow(options: CreateEscrowOptions) {
    if (!options.jobId || options.jobId.trim().length === 0) {
      throw new ValidationError("jobId cannot be empty");
    }
    if (!isValidStellarAddress(options.client)) {
      throw new ValidationError(`Invalid client address: ${options.client}`);
    }
    if (!isValidStellarAddress(options.freelancer)) {
      throw new ValidationError(`Invalid freelancer address: ${options.freelancer}`);
    }
    if (!isValidStellarAddress(options.arbitratorAddress)) {
      throw new ValidationError(`Invalid arbitrator address: ${options.arbitratorAddress}`);
    }
    if (options.amount <= 0n) {
      throw new ValidationError("Amount must be greater than zero");
    }

    const now = Math.floor(Date.now() / 1000);
    if (options.timeoutTimestamp <= now) {
      throw new ValidationError("timeoutTimestamp must be in the future");
    }

    validateMilestones(options.milestones);

    return {
      contractId: this.contractId,
      method: "create_escrow",
      args: {
        job_id: options.jobId,
        client: options.client,
        params: {
          freelancer: options.freelancer,
          token: options.tokenAddress,
          total_amount: options.amount,
          timeout_timestamp: options.timeoutTimestamp,
          arbitrator: options.arbitratorAddress,
        },
        milestones: (options.milestones || []).map((m) => ({
          description: m.description,
          percentage: m.percentage,
        })),
      },
    };
  }

  /**
   * Prepare parameters for starting work.
   */
  public prepareStartWork(jobId: string, freelancerAddress: string) {
    if (!jobId) throw new ValidationError("jobId is required");
    if (!isValidStellarAddress(freelancerAddress)) {
      throw new ValidationError(`Invalid freelancer address: ${freelancerAddress}`);
    }

    return {
      contractId: this.contractId,
      method: "start_work",
      args: {
        job_id: jobId,
        freelancer: freelancerAddress,
      },
    };
  }

  /**
   * Prepare parameters for submitting deliverable proof.
   */
  public prepareSubmitDeliverable(jobId: string, freelancerAddress: string, deliverableHashHex: string) {
    if (!jobId) throw new ValidationError("jobId is required");
    if (!isValidStellarAddress(freelancerAddress)) {
      throw new ValidationError(`Invalid freelancer address: ${freelancerAddress}`);
    }
    if (!/^[0-9a-fA-F]{64}$/.test(deliverableHashHex)) {
      throw new ValidationError("deliverableHashHex must be a 32-byte hexadecimal string (64 characters)");
    }

    return {
      contractId: this.contractId,
      method: "submit_deliverable",
      args: {
        job_id: jobId,
        freelancer: freelancerAddress,
        deliverable_hash: deliverableHashHex,
      },
    };
  }

  /**
   * Prepare parameters for submitting deliverable proof for a specific milestone ID.
   */
  public prepareSubmitMilestoneDeliverable(
    jobId: string,
    freelancerAddress: string,
    milestoneId: number,
    deliverableHashHex: string
  ) {
    if (!jobId) throw new ValidationError("jobId is required");
    if (!isValidStellarAddress(freelancerAddress)) {
      throw new ValidationError(`Invalid freelancer address: ${freelancerAddress}`);
    }
    if (!Number.isInteger(milestoneId) || milestoneId <= 0) {
      throw new ValidationError("milestoneId must be a positive integer");
    }
    if (!/^[0-9a-fA-F]{64}$/.test(deliverableHashHex)) {
      throw new ValidationError("deliverableHashHex must be a 32-byte hexadecimal string (64 characters)");
    }

    return {
      contractId: this.contractId,
      method: "submit_milestone_deliverable",
      args: {
        job_id: jobId,
        freelancer: freelancerAddress,
        milestone_id: milestoneId,
        deliverable_hash: deliverableHashHex,
      },
    };
  }

  /**
   * Prepare parameters for approving and releasing a milestone payout.
   */
  public prepareReleaseMilestone(jobId: string, clientAddress: string, milestoneId: number) {
    if (!jobId) throw new ValidationError("jobId is required");
    if (!isValidStellarAddress(clientAddress)) {
      throw new ValidationError(`Invalid client address: ${clientAddress}`);
    }
    if (milestoneId <= 0) {
      throw new ValidationError("milestoneId must be a positive integer");
    }

    return {
      contractId: this.contractId,
      method: "release_milestone",
      args: {
        job_id: jobId,
        client: clientAddress,
        milestone_id: milestoneId,
      },
    };
  }

  /**
   * Prepare parameters for releasing a partial payout of a milestone.
   */
  public prepareReleasePartialMilestone(
    jobId: string,
    clientAddress: string,
    milestoneId: number,
    amount: bigint | number | string
  ) {
    if (!jobId) throw new ValidationError("jobId is required");
    if (!isValidStellarAddress(clientAddress)) {
      throw new ValidationError(`Invalid client address: ${clientAddress}`);
    }
    if (!Number.isInteger(milestoneId) || milestoneId <= 0) {
      throw new ValidationError("milestoneId must be a positive integer");
    }

    let amountStroops: bigint;
    if (typeof amount === "bigint") {
      if (amount <= 0n) {
        throw new ValidationError("Partial release amount must be greater than zero");
      }
      amountStroops = amount;
    } else {
      amountStroops = toStroops(amount);
      if (amountStroops <= 0n) {
        throw new ValidationError("Partial release amount must be greater than zero");
      }
    }

    return {
      contractId: this.contractId,
      method: "release_partial_milestone",
      args: {
        job_id: jobId,
        client: clientAddress,
        milestone_id: milestoneId,
        amount: amountStroops,
      },
    };
  }

  /**
   * Prepare parameters for setting a milestone completion deadline timestamp.
   */
  public prepareSetMilestoneDeadline(
    jobId: string,
    clientAddress: string,
    milestoneId: number,
    deadlineTimestamp: number
  ) {
    if (!jobId) throw new ValidationError("jobId is required");
    if (!isValidStellarAddress(clientAddress)) {
      throw new ValidationError(`Invalid client address: ${clientAddress}`);
    }
    if (!Number.isInteger(milestoneId) || milestoneId <= 0) {
      throw new ValidationError("milestoneId must be a positive integer");
    }
    if (!Number.isInteger(deadlineTimestamp) || deadlineTimestamp <= 0) {
      throw new ValidationError("deadlineTimestamp must be a positive integer");
    }

    return {
      contractId: this.contractId,
      method: "set_milestone_deadline",
      args: {
        job_id: jobId,
        client: clientAddress,
        milestone_id: milestoneId,
        deadline: BigInt(deadlineTimestamp),
      },
    };
  }

  /**
   * Prepare parameters for cancelling an overdue milestone and claiming a refund.
   */
  public prepareCancelExpiredMilestone(
    jobId: string,
    clientAddress: string,
    milestoneId: number
  ) {
    if (!jobId) throw new ValidationError("jobId is required");
    if (!isValidStellarAddress(clientAddress)) {
      throw new ValidationError(`Invalid client address: ${clientAddress}`);
    }
    if (!Number.isInteger(milestoneId) || milestoneId <= 0) {
      throw new ValidationError("milestoneId must be a positive integer");
    }

    return {
      contractId: this.contractId,
      method: "cancel_expired_milestone",
      args: {
        job_id: jobId,
        client: clientAddress,
        milestone_id: milestoneId,
      },
    };
  }

  /**
   * Prepare parameters for extending an unapproved milestone deadline.
   */
  public prepareExtendMilestoneDeadline(
    jobId: string,
    clientAddress: string,
    milestoneId: number,
    extensionSeconds: number
  ) {
    if (!jobId) throw new ValidationError("jobId is required");
    if (!isValidStellarAddress(clientAddress)) {
      throw new ValidationError(`Invalid client address: ${clientAddress}`);
    }
    if (!Number.isInteger(milestoneId) || milestoneId <= 0) {
      throw new ValidationError("milestoneId must be a positive integer");
    }
    if (!Number.isInteger(extensionSeconds) || extensionSeconds <= 0) {
      throw new ValidationError("extensionSeconds must be a positive integer");
    }

    return {
      contractId: this.contractId,
      method: "extend_milestone_deadline",
      args: {
        job_id: jobId,
        client: clientAddress,
        milestone_id: milestoneId,
        extension_seconds: BigInt(extensionSeconds),
      },
    };
  }

  /**
   * Prepare parameters for claiming timeout refund.
   */
  public prepareClaimTimeoutRefund(jobId: string) {
    if (!jobId) throw new ValidationError("jobId is required");

    return {
      contractId: this.contractId,
      method: "claim_timeout_refund",
      args: {
        job_id: jobId,
      },
    };
  }

  /**
   * Prepare parameters for raising a dispute.
   */
  public prepareRaiseDispute(jobId: string, callerAddress: string) {
    if (!jobId) throw new ValidationError("jobId is required");
    if (!isValidStellarAddress(callerAddress)) {
      throw new ValidationError(`Invalid caller address: ${callerAddress}`);
    }

    return {
      contractId: this.contractId,
      method: "raise_dispute",
      args: {
        job_id: jobId,
        caller: callerAddress,
      },
    };
  }

  /**
   * Prepare parameters for submitting cryptographic evidence hash during active dispute.
   */
  public prepareSubmitDisputeEvidence(
    jobId: string,
    callerAddress: string,
    evidenceHashHex: string
  ) {
    if (!jobId || jobId.trim().length === 0) {
      throw new ValidationError("jobId is required");
    }
    if (!isValidStellarAddress(callerAddress)) {
      throw new ValidationError(`Invalid caller address: ${callerAddress}`);
    }
    if (!/^[0-9a-fA-F]{64}$/.test(evidenceHashHex)) {
      throw new ValidationError("evidenceHashHex must be a 32-byte hexadecimal string (64 characters)");
    }

    return {
      contractId: this.contractId,
      method: "submit_dispute_evidence",
      args: {
        job_id: jobId,
        caller: callerAddress,
        evidence_hash: evidenceHashHex,
      },
    };
  }

  /**
   * Prepare parameters for arbitrator dispute resolution.
   */
  public prepareResolveDispute(
    jobId: string,
    arbitratorAddress: string,
    clientAward: bigint,
    freelancerAward: bigint
  ) {
    if (!jobId) throw new ValidationError("jobId is required");
    if (!isValidStellarAddress(arbitratorAddress)) {
      throw new ValidationError(`Invalid arbitrator address: ${arbitratorAddress}`);
    }
    if (clientAward < 0n || freelancerAward < 0n) {
      throw new ValidationError("Awards cannot be negative");
    }

    return {
      contractId: this.contractId,
      method: "resolve_dispute",
      args: {
        job_id: jobId,
        arbitrator: arbitratorAddress,
        client_award: clientAward,
        freelancer_award: freelancerAward,
      },
    };
  }

  /**
   * Prepare parameters for casting an arbitrator dispute vote in a 2-of-3 quorum panel.
   */
  public prepareVoteDispute(
    jobId: string,
    arbitratorAddress: string,
    clientAward: bigint,
    freelancerAward: bigint
  ) {
    if (!jobId || jobId.trim().length === 0) {
      throw new ValidationError("jobId is required");
    }
    if (!isValidStellarAddress(arbitratorAddress)) {
      throw new ValidationError(`Invalid arbitrator address: ${arbitratorAddress}`);
    }
    if (clientAward < 0n || freelancerAward < 0n) {
      throw new ValidationError("Awards cannot be negative");
    }

    return {
      contractId: this.contractId,
      method: "vote_dispute",
      args: {
        job_id: jobId,
        arbitrator: arbitratorAddress,
        client_award: clientAward,
        freelancer_award: freelancerAward,
      },
    };
  }

  /**
   * Prepare parameters for emergency pause (admin only).
   */
  public preparePause() {
    return {
      contractId: this.contractId,
      method: "pause",
      args: {},
    };
  }

  /**
   * Prepare parameters for unpause (admin only).
   */
  public prepareUnpause() {
    return {
      contractId: this.contractId,
      method: "unpause",
      args: {},
    };
  }

  /**
   * Prepare parameters for mutual cancellation.
   */
  public prepareMutualCancel(jobId: string) {
    if (!jobId || jobId.trim().length === 0) {
      throw new ValidationError("jobId is required");
    }

    return {
      contractId: this.contractId,
      method: "mutual_cancel",
      args: {
        job_id: jobId,
      },
    };
  }

  /**
   * Prepare parameters for updating platform fee bps (admin only).
   */
  public prepareSetFeeBps(feeBps: number) {
    if (!Number.isInteger(feeBps) || feeBps < 0 || feeBps > 1000) {
      throw new ValidationError("feeBps must be an integer between 0 and 1000 (max 10.00%)");
    }

    return {
      contractId: this.contractId,
      method: "set_fee_bps",
      args: {
        fee_bps: feeBps,
      },
    };
  }

  /**
   * Prepare parameters for rotating treasury recipient address (admin only).
   */
  public prepareSetTreasury(newTreasuryAddress: string) {
    if (!isValidStellarAddress(newTreasuryAddress)) {
      throw new ValidationError(`Invalid treasury address: ${newTreasuryAddress}`);
    }

    return {
      contractId: this.contractId,
      method: "set_treasury",
      args: {
        new_treasury: newTreasuryAddress,
      },
    };
  }

  /**
   * Prepare parameters for transferring admin rights (admin only).
   */
  public prepareSetAdmin(newAdminAddress: string) {
    if (!isValidStellarAddress(newAdminAddress)) {
      throw new ValidationError(`Invalid new admin address: ${newAdminAddress}`);
    }

    return {
      contractId: this.contractId,
      method: "set_admin",
      args: {
        new_admin: newAdminAddress,
      },
    };
  }

  /**
   * Prepare parameters for depositing an arbitrator staking bond.
   */
  public prepareRegisterArbitrator(
    arbitratorAddress: string,
    tokenAddress: string,
    bondAmount: bigint
  ) {
    if (!isValidStellarAddress(arbitratorAddress)) {
      throw new ValidationError(`Invalid arbitrator address: ${arbitratorAddress}`);
    }
    if (!isValidStellarAddress(tokenAddress)) {
      throw new ValidationError(`Invalid token address: ${tokenAddress}`);
    }
    if (bondAmount <= 0n) {
      throw new ValidationError("Bond amount must be strictly positive");
    }

    return {
      contractId: this.contractId,
      method: "register_arbitrator",
      args: {
        arbitrator: arbitratorAddress,
        token: tokenAddress,
        bond_amount: bondAmount,
      },
    };
  }

  /**
   * Prepare parameters for slashing an arbitrator's bond for misconduct (admin only).
   */
  public prepareSlashArbitrator(
    adminAddress: string,
    arbitratorAddress: string,
    recipientAddress: string,
    tokenAddress: string,
    slashAmount: bigint
  ) {
    if (!isValidStellarAddress(adminAddress)) {
      throw new ValidationError(`Invalid admin address: ${adminAddress}`);
    }
    if (!isValidStellarAddress(arbitratorAddress)) {
      throw new ValidationError(`Invalid arbitrator address: ${arbitratorAddress}`);
    }
    if (!isValidStellarAddress(recipientAddress)) {
      throw new ValidationError(`Invalid recipient address: ${recipientAddress}`);
    }
    if (!isValidStellarAddress(tokenAddress)) {
      throw new ValidationError(`Invalid token address: ${tokenAddress}`);
    }
    if (slashAmount <= 0n) {
      throw new ValidationError("Slash amount must be strictly positive");
    }

    return {
      contractId: this.contractId,
      method: "slash_arbitrator_bond",
      args: {
        admin: adminAddress,
        arbitrator: arbitratorAddress,
        recipient: recipientAddress,
        token: tokenAddress,
        slash_amount: slashAmount,
      },
    };
  }

  /**
   * Estimates resource costs and inclusion fees from a Soroban simulation result.
   */
  public estimateResources(
    simulationResponse: any,
    options?: {
      baseInclusionFeeStroops?: number;
      budgetLimits?: ResourceBudgetLimits;
    }
  ): ResourceCost {
    return parseResourceEstimate(simulationResponse, options);
  }

  /**
   * Signs an XDR transaction envelope using the Freighter wallet with this client's network.
   */
  public async signWithFreighter(
    xdr: string,
    options?: Omit<FreighterSignOptions, "networkPassphrase">,
    customApi?: FreighterApi
  ): Promise<string> {
    return signWithFreighter(
      xdr,
      {
        ...options,
        networkPassphrase: this.networkPassphrase,
      },
      customApi
    );
  }

  /**
   * Executes an asynchronous RPC operation with automatic exponential backoff on transient errors.
   */
  public async invokeWithRetry<T>(
    operation: () => Promise<T>,
    retryOptions?: RpcRetryOptions
  ): Promise<T> {
    return withRpcRetry(operation, retryOptions);
  }

  /**
   * Probes contract availability, RPC ping latency, and latest ledger sequence.
   *
   * @param customFetcher - Optional custom fetcher for testing
   */
  public async getContractHealth(
    customFetcher?: (rpcUrl: string, payload: any) => Promise<any>
  ): Promise<ContractHealthReport> {
    const start = Date.now();
    try {
      const payload = {
        jsonrpc: "2.0",
        id: 1,
        method: "getLatestLedger",
        params: {},
      };

      let resData: any;
      if (customFetcher) {
        resData = await customFetcher(this.rpcUrl, payload);
      } else {
        const response = await fetch(this.rpcUrl, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
        });
        resData = await response.json();
      }

      const latencyMs = Date.now() - start;

      if (resData?.error) {
        return {
          contractId: this.contractId,
          isResponsive: false,
          isPaused: false,
          rpcLatencyMs: latencyMs,
          error: resData.error.message || "RPC returned error",
        };
      }

      return {
        contractId: this.contractId,
        isResponsive: true,
        isPaused: false,
        rpcLatencyMs: latencyMs,
        latestLedgerSequence: resData?.result?.sequence,
      };
    } catch (err: any) {
      return {
        contractId: this.contractId,
        isResponsive: false,
        isPaused: false,
        rpcLatencyMs: Date.now() - start,
        error: err?.message || "Failed to contact Soroban RPC endpoint",
      };
    }
  }

  /**
   * Partitions batch payment payouts conforming to the Stellar transaction operation limits.
   *
   * @param payments - List of payment instructions
   * @param options - Chunk partitioning and fee options
   * @returns Partitioned batch summary
   */
  public prepareBatchPayments(
    payments: PaymentItem[],
    options?: PartitionOptions
  ): BatchPaymentSummary {
    return partitionPayments(payments, options);
  }
}
