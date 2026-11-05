import { StrKey } from "@stellar/stellar-sdk";
import { ValidationError } from "./errors";
import { MilestoneInput } from "./types";

/**
 * Validate that milestone percentages total exactly 100% and meet boundary conditions.
 */
export function validateMilestones(milestones?: MilestoneInput[]): void {
  if (!milestones || milestones.length === 0) {
    return;
  }

  if (milestones.length > 25) {
    throw new ValidationError(`Milestone count cannot exceed 25, got ${milestones.length}`);
  }

  let total = 0;
  for (const m of milestones) {
    if (!m.description || m.description.trim().length === 0) {
      throw new ValidationError("Milestone description cannot be empty");
    }
    if (m.description.length > 256) {
      throw new ValidationError("Milestone description exceeds 256 characters");
    }
    if (!Number.isInteger(m.percentage) || m.percentage <= 0 || m.percentage > 100) {
      throw new ValidationError(`Milestone percentage must be an integer between 1 and 100, got ${m.percentage}`);
    }
    total += m.percentage;
  }

  if (total !== 100) {
    throw new ValidationError(`Total milestone percentages must equal exactly 100%, got ${total}%`);
  }
}

/**
 * Convert human-readable token amount to atomic units without precision loss or truncation.
 * Deliberately rejects excess precision, negatives, and invalid decimal notation rather than silently truncating.
 */
export function toAtomicUnits(amount: number | string, decimals = 7): bigint {
  if (!Number.isInteger(decimals) || decimals < 0 || decimals > 18) {
    throw new ValidationError("Invalid decimal precision (must be integer 0-18)");
  }

  const strAmount = typeof amount === "number" ? amount.toString() : String(amount).trim();

  // Strict positive decimal regex: rejects negative, scientific notation, and malformed strings
  if (!/^(0|[1-9]\d*)(\.\d+)?$/.test(strAmount)) {
    throw new ValidationError("Invalid monetary amount: must be a positive decimal number without scientific notation");
  }

  const [whole, fraction = ""] = strAmount.split(".");
  if (fraction.length > decimals) {
    throw new ValidationError(
      `Excess decimal precision: amount '${strAmount}' exceeds maximum of ${decimals} decimal places`
    );
  }

  const atomic = BigInt(whole + fraction.padEnd(decimals, "0"));
  if (atomic <= 0n) {
    throw new ValidationError("Amount must be strictly positive");
  }

  return atomic;
}

/**
 * Convert atomic units to human-readable string.
 */
export function fromAtomicUnits(atomic: bigint, decimals = 7): string {
  if (atomic < 0n) {
    throw new ValidationError("Atomic units cannot be negative");
  }
  const str = atomic.toString().padStart(decimals + 1, "0");
  const whole = str.slice(0, str.length - decimals);
  const frac = str.slice(str.length - decimals).replace(/0+$/, "");
  return frac.length > 0 ? `${whole}.${frac}` : whole;
}

/**
 * Validate that a Stellar address is well-formed with valid StrKey checksum
 * (Ed25519 public key starting with G or Soroban contract address starting with C).
 */
export function isValidStellarAddress(address: string): boolean {
  if (!address || typeof address !== "string") return false;
  return StrKey.isValidEd25519PublicKey(address) || StrKey.isValidContract(address);
}

/**
 * Validate specifically that an address is a valid Soroban contract address (C...).
 */
export function isValidContractAddress(address: string): boolean {
  if (!address || typeof address !== "string") return false;
  return StrKey.isValidContract(address);
}
