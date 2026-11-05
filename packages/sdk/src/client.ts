import {
  CreateEscrowOptions,
  EscrowConfig,
  EscrowRecord,
  EscrowStatus,
  Milestone,
} from "./types";
import { validateMilestones, isValidStellarAddress } from "./utils";
import { ValidationError } from "./errors";

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
}
