import { createHash } from "crypto";
import { ValidationError } from "./errors";

const WASM_HASH_REGEX = /^[0-9a-fA-F]{64}$/;

export interface StagedUpgradeInfo {
  newWasmHash: string;
  unlockTimestamp: number;
  delaySeconds: number;
  isUnlockMature: boolean;
  remainingSeconds: number;
}

export interface GovernanceQuorumStatus {
  quorumMet: boolean;
  passed: boolean;
  totalVotes: bigint;
  forPercentage: number;
}

/**
 * Validates whether a given string is a valid 64-character hex WASM hash.
 */
export function validateWasmHash(hashHex: string): string {
  if (!hashHex || typeof hashHex !== "string") {
    throw new ValidationError("WASM hash must be a non-empty string");
  }
  const trimmed = hashHex.trim().toLowerCase();
  if (!WASM_HASH_REGEX.test(trimmed)) {
    throw new ValidationError("WASM hash must be a valid 64-character hexadecimal string");
  }
  return trimmed;
}

/**
 * Computes SHA-256 hash of compiled Soroban WASM binary bytecode.
 */
export function computeWasmHash(wasmBytes: Uint8Array | Buffer): string {
  if (!wasmBytes || wasmBytes.length === 0) {
    throw new ValidationError("WASM binary cannot be empty");
  }
  return createHash("sha256").update(wasmBytes).digest("hex").toLowerCase();
}

/**
 * Computes timelock maturity and remaining countdown seconds.
 */
export function calculateTimelockStatus(
  unlockTimestamp: number,
  currentTimestamp: number = Math.floor(Date.now() / 1000)
): { isUnlockMature: boolean; remainingSeconds: number } {
  if (typeof unlockTimestamp !== "number" || isNaN(unlockTimestamp)) {
    throw new ValidationError("unlockTimestamp must be a valid numeric timestamp");
  }
  const remaining = Math.max(0, unlockTimestamp - currentTimestamp);
  return {
    isUnlockMature: remaining === 0,
    remainingSeconds: remaining,
  };
}

/**
 * Evaluates whether governance votes satisfy quorum and majority threshold.
 */
export function evaluateGovernanceQuorum(
  votesFor: bigint | number | string,
  votesAgainst: bigint | number | string,
  quorumThreshold: bigint | number | string
): GovernanceQuorumStatus {
  const vFor = BigInt(votesFor);
  const vAgainst = BigInt(votesAgainst);
  const quorum = BigInt(quorumThreshold);

  if (vFor < 0n || vAgainst < 0n || quorum <= 0n) {
    throw new ValidationError("Vote counts and quorum threshold must be positive values");
  }

  const totalVotes = vFor + vAgainst;
  const quorumMet = totalVotes >= quorum;
  const passed = quorumMet && vFor > vAgainst;

  const forPercentage =
    totalVotes === 0n ? 0 : Number((vFor * 10000n) / totalVotes) / 100;

  return {
    quorumMet,
    passed,
    totalVotes,
    forPercentage,
  };
}

/**
 * Helper class for managing contract upgrade workflows and validation.
 */
export class UpgradeManager {
  /**
   * Prepares and validates parameters for staging a contract upgrade.
   */
  public static prepareUpgrade(
    wasmBytesOrHash: Uint8Array | Buffer | string,
    delaySeconds: number,
    currentTimestamp: number = Math.floor(Date.now() / 1000)
  ): StagedUpgradeInfo {
    if (delaySeconds < 3600) {
      throw new ValidationError("Upgrade delay cannot be less than 3,600 seconds (1 hour)");
    }

    const newWasmHash =
      typeof wasmBytesOrHash === "string"
        ? validateWasmHash(wasmBytesOrHash)
        : computeWasmHash(wasmBytesOrHash);

    const unlockTimestamp = currentTimestamp + delaySeconds;
    const { isUnlockMature, remainingSeconds } = calculateTimelockStatus(
      unlockTimestamp,
      currentTimestamp
    );

    return {
      newWasmHash,
      unlockTimestamp,
      delaySeconds,
      isUnlockMature,
      remainingSeconds,
    };
  }
}
