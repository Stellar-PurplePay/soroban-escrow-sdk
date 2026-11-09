import { createHash } from "crypto";
import { ValidationError } from "./errors";

const HEX_64_REGEX = /^[0-9a-fA-F]{64}$/;

/**
 * Computes a standard 32-byte SHA-256 hexadecimal digest for deliverable files or text proofs.
 *
 * @param input - File buffer, Uint8Array, or string content
 * @returns 64-character lowercase hex string
 */
export function createDeliverableHash(input: string | Uint8Array): string {
  if (input === null || input === undefined) {
    throw new ValidationError("Input for deliverable hash cannot be null or undefined");
  }

  const hasher = createHash("sha256");
  if (typeof input === "string") {
    if (input.length === 0) {
      throw new ValidationError("Input string cannot be empty");
    }
    hasher.update(input, "utf8");
  } else {
    if (input.length === 0) {
      throw new ValidationError("Input buffer cannot be empty");
    }
    hasher.update(input);
  }

  return hasher.digest("hex").toLowerCase();
}

/**
 * Verifies whether an input matches an expected 32-byte deliverable hash.
 *
 * @param input - Deliverable data
 * @param expectedHashHex - 64-character hex hash from contract
 * @returns true if hashes match
 */
export function verifyDeliverableHash(input: string | Uint8Array, expectedHashHex: string): boolean {
  if (!expectedHashHex || !HEX_64_REGEX.test(expectedHashHex)) {
    throw new ValidationError("expectedHashHex must be a valid 64-character hexadecimal string");
  }

  try {
    const computed = createDeliverableHash(input);
    return computed === expectedHashHex.toLowerCase();
  } catch {
    return false;
  }
}
