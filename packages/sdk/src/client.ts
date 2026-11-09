import {
  CreateEscrowOptions,
  EscrowConfig,
  EscrowRecord,
  EscrowStatus,
  Milestone,
} from "./types";
import { validateMilestones, isValidStellarAddress } from "./utils";
import { ValidationError } from "./errors";
import { parseResourceEstimate, ResourceCost, ResourceBudgetLimits } from "./estimator";
import { signWithFreighter, FreighterSignOptions, FreighterApi } from "./freighter";
import { withRpcRetry, RpcRetryOptions } from "./rpcRetry";

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
}
