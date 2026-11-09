import { createHash } from "crypto";
import { ValidationError } from "./errors";

export interface AuditEntry {
  index: number;
  action: string;
  timestamp: string | number;
  payload: any;
  previousHash: string;
  entryHash: string;
}

export interface AuditVerificationResult {
  isValid: boolean;
  brokenIndex?: number;
  expectedHash?: string;
  actualHash?: string;
  totalEntries: number;
  rootHash?: string;
  error?: string;
}

const GENESIS_PREV_HASH = "0".repeat(64);

/**
 * Computes canonical deterministic SHA-256 hash for an audit log entry.
 */
export function computeAuditEntryHash(
  entry: Omit<AuditEntry, "entryHash">
): string {
  if (entry.index === undefined || entry.index === null) {
    throw new ValidationError("Audit entry index is required");
  }
  if (!entry.action) {
    throw new ValidationError("Audit entry action is required");
  }

  const canonicalPayload =
    typeof entry.payload === "string"
      ? entry.payload
      : JSON.stringify(entry.payload || {});

  const canonical = JSON.stringify({
    index: entry.index,
    action: entry.action,
    timestamp: String(entry.timestamp),
    payload: canonicalPayload,
    previousHash: (entry.previousHash || GENESIS_PREV_HASH).toLowerCase(),
  });

  return createHash("sha256").update(canonical, "utf8").digest("hex").toLowerCase();
}

/**
 * Validates a tamper-evident chain of audit log entries.
 *
 * @param chain - Array of audit log entries in sequential order
 * @param expectedRootHash - Optional expected hash of the latest entry
 * @returns Verification result including broken index if tampering is detected
 */
export function verifyAuditChain(
  chain: AuditEntry[],
  expectedRootHash?: string
): AuditVerificationResult {
  if (!Array.isArray(chain)) {
    throw new ValidationError("Audit chain must be an array");
  }

  if (chain.length === 0) {
    return {
      isValid: true,
      totalEntries: 0,
    };
  }

  for (let i = 0; i < chain.length; i++) {
    const current = chain[i];

    // Check monotonic indexing (must start at 1 or 0 and strictly increment by 1)
    if (i > 0 && current.index !== chain[i - 1].index + 1) {
      return {
        isValid: false,
        brokenIndex: current.index,
        error: `Non-sequential index at entry ${i}: expected ${chain[i - 1].index + 1}, got ${current.index}`,
        totalEntries: chain.length,
      };
    }

    // Check previous hash linkage
    if (i === 0) {
      const prev = (current.previousHash || "").toLowerCase();
      if (prev !== "" && prev !== GENESIS_PREV_HASH) {
        return {
          isValid: false,
          brokenIndex: current.index,
          error: `Genesis entry previousHash must be empty or 64 zeros, got ${current.previousHash}`,
          totalEntries: chain.length,
        };
      }
    } else {
      const prevHash = (current.previousHash || "").toLowerCase();
      const expectedPrevHash = (chain[i - 1].entryHash || "").toLowerCase();
      if (prevHash !== expectedPrevHash) {
        return {
          isValid: false,
          brokenIndex: current.index,
          expectedHash: expectedPrevHash,
          actualHash: prevHash,
          error: `Broken chain link at index ${current.index}: previousHash does not match prior entryHash`,
          totalEntries: chain.length,
        };
      }
    }

    // Verify cryptographic self-consistency
    const calculatedHash = computeAuditEntryHash(current);
    if ((current.entryHash || "").toLowerCase() !== calculatedHash) {
      return {
        isValid: false,
        brokenIndex: current.index,
        expectedHash: calculatedHash,
        actualHash: current.entryHash,
        error: `Tampered hash at index ${current.index}: recomputed hash does not match stored entryHash`,
        totalEntries: chain.length,
      };
    }
  }

  const finalEntry = chain[chain.length - 1];
  const rootHash = finalEntry.entryHash.toLowerCase();

  if (expectedRootHash && expectedRootHash.toLowerCase() !== rootHash) {
    return {
      isValid: false,
      brokenIndex: finalEntry.index,
      expectedHash: expectedRootHash.toLowerCase(),
      actualHash: rootHash,
      error: `Root hash mismatch: expected ${expectedRootHash}, got ${rootHash}`,
      totalEntries: chain.length,
      rootHash,
    };
  }

  return {
    isValid: true,
    totalEntries: chain.length,
    rootHash,
  };
}

/**
 * In-memory tamper-evident audit chain inspector & builder.
 */
export class AuditChainInspector {
  private chain: AuditEntry[] = [];

  constructor(initialChain?: AuditEntry[]) {
    if (initialChain) {
      this.chain = [...initialChain];
    }
  }

  public append(action: string, payload: any, timestamp: string | number = Date.now()): AuditEntry {
    const index = this.chain.length + 1;
    const previousHash =
      this.chain.length === 0
        ? GENESIS_PREV_HASH
        : this.chain[this.chain.length - 1].entryHash;

    const baseEntry = {
      index,
      action,
      timestamp,
      payload,
      previousHash,
    };

    const entryHash = computeAuditEntryHash(baseEntry);
    const newEntry: AuditEntry = { ...baseEntry, entryHash };
    this.chain.push(newEntry);
    return newEntry;
  }

  public getChain(): ReadonlyArray<AuditEntry> {
    return this.chain;
  }

  public getRootHash(): string | null {
    if (this.chain.length === 0) return null;
    return this.chain[this.chain.length - 1].entryHash;
  }

  public verify(expectedRootHash?: string): AuditVerificationResult {
    return verifyAuditChain(this.chain, expectedRootHash);
  }
}
