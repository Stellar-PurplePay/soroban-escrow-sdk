/**
 * packages/sdk/src/utils/stroopPrecision.ts
 *
 * High-precision Stroop conversions preventing IEEE-754 floating-point inaccuracies.
 * Stellar Stroop is 1 / 10,000,000th of an XLM (7 decimal places).
 */

import { ValidationError } from "../errors";

export const STROOPS_PER_XLM_BIGINT = 10_000_000n;
const MAX_STROOPS = 9223372036854775807n; // Int64 max

/**
 * Converts a decimal XLM string or number to exact BigInt Stroops.
 *
 * @param amount - e.g. "1.5000000" or "0.0000100"
 * @returns BigInt stroop amount
 */
export function toStroops(amount: string | number): bigint {
  const str = String(amount).trim();

  if (!str) {
    throw new ValidationError("Amount cannot be empty");
  }

  // Reject scientific notation
  if (/[eE]/.test(str)) {
    throw new ValidationError(`Scientific notation is not permitted: "${str}"`);
  }

  // Reject negative numbers
  if (str.startsWith("-")) {
    throw new ValidationError(`Negative amounts are not permitted: "${str}"`);
  }

  if (!/^\d+(\.\d+)?$/.test(str)) {
    throw new ValidationError(`Invalid numeric format for XLM amount: "${str}"`);
  }

  const [whole, fractional = ""] = str.split(".");

  if (fractional.length > 7) {
    throw new ValidationError(
      `Amount "${str}" exceeds Stellar's maximum 7 decimal digits of precision`
    );
  }

  const paddedFractional = fractional.padEnd(7, "0");
  const wholeStroops = BigInt(whole) * STROOPS_PER_XLM_BIGINT;
  const fractionalStroops = BigInt(paddedFractional);
  const totalStroops = wholeStroops + fractionalStroops;

  if (totalStroops > MAX_STROOPS) {
    throw new ValidationError(`Amount exceeds maximum allowable 64-bit Stroop integer limit`);
  }

  return totalStroops;
}

/**
 * Converts BigInt Stroops back to a 7-decimal formatted XLM string.
 *
 * @param stroops - BigInt or integer number of stroops
 * @returns Formatted XLM string
 */
export function fromStroops(stroops: bigint | number | string): string {
  let val: bigint;
  try {
    val = BigInt(stroops);
  } catch {
    throw new ValidationError(`Invalid stroop value: "${stroops}"`);
  }

  if (val < 0n) {
    throw new ValidationError(`Stroop value cannot be negative: ${val}`);
  }

  const whole = val / STROOPS_PER_XLM_BIGINT;
  const remainder = val % STROOPS_PER_XLM_BIGINT;
  const paddedRemainder = remainder.toString().padStart(7, "0");

  return `${whole}.${paddedRemainder}`;
}
