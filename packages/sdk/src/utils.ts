import { ValidationError } from "./errors";
import { MilestoneInput } from "./types";

/**
 * Validate that milestone percentages total exactly 100%.
 */
export function validateMilestones(milestones?: MilestoneInput[]): void {
  if (!milestones || milestones.length === 0) {
    return;
  }

  let total = 0;
  for (const m of milestones) {
    if (m.percentage <= 0 || m.percentage > 100) {
      throw new ValidationError(`Milestone percentage must be between 1 and 100, got ${m.percentage}`);
    }
    total += m.percentage;
  }

  if (total !== 100) {
    throw new ValidationError(`Total milestone percentages must equal exactly 100%, got ${total}%`);
  }
}

/**
 * Convert human-readable token amount to atomic units (e.g. 7 decimal places for XLM/USDC).
 */
export function toAtomicUnits(amount: number | string, decimals = 7): bigint {
  const [whole, fraction = ""] = String(amount).split(".");
  const paddedFraction = fraction.padEnd(decimals, "0").slice(0, decimals);
  return BigInt(whole + paddedFraction);
}

/**
 * Convert atomic units to human-readable string.
 */
export function fromAtomicUnits(atomic: bigint, decimals = 7): string {
  const str = atomic.toString().padStart(decimals + 1, "0");
  const whole = str.slice(0, str.length - decimals);
  const frac = str.slice(str.length - decimals).replace(/0+$/, "");
  return frac.length > 0 ? `${whole}.${frac}` : whole;
}

/**
 * Validate that a Stellar address is well-formed (starts with G for account or C for contract).
 */
export function isValidStellarAddress(address: string): boolean {
  return /^[GC][A-Z0-9]{55}$/.test(address);
}
