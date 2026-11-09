import { StrKey } from "@stellar/stellar-sdk";

export interface ValidationResult<T = string> {
  isValid: boolean;
  sanitized?: T;
  error?: string;
}

/**
 * Sanitizes generic user-supplied strings by stripping null bytes and HTML/script injection vectors.
 */
export function sanitizeString(input: string): string {
  if (typeof input !== "string") return "";
  return input
    .replace(/\0/g, "")
    .replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, "")
    .replace(/\bon\w+\s*=/gi, "")
    .replace(/javascript:/gi, "")
    .replace(/\s+/g, " ")
    .trim();
}

/**
 * Validates and sanitizes a Stellar transaction memo string (max 28 UTF-8 bytes).
 */
export function validateAndSanitizeMemo(memo: string): ValidationResult<string> {
  const sanitized = sanitizeString(memo);
  const byteLength = Buffer.byteLength(sanitized, "utf8");

  if (byteLength > 28) {
    return {
      isValid: false,
      error: `Memo exceeds Stellar maximum of 28 UTF-8 bytes (current: ${byteLength} bytes)`,
    };
  }

  return {
    isValid: true,
    sanitized,
  };
}

/**
 * Verifies Stellar StrKey validity (Ed25519 Public Key 'G...', Contract ID 'C...', or Muxed Account 'M...').
 */
export function verifyStellarKey(address: string): ValidationResult<string> {
  if (!address || typeof address !== "string") {
    return { isValid: false, error: "Address must be a non-empty string" };
  }

  const trimmed = address.trim();

  // Ed25519 Public Key
  if (trimmed.startsWith("G")) {
    const isValid = StrKey.isValidEd25519PublicKey(trimmed);
    return isValid
      ? { isValid: true, sanitized: trimmed }
      : { isValid: false, error: "Invalid Stellar Ed25519 public key format or checksum" };
  }

  // Contract ID
  if (trimmed.startsWith("C")) {
    const isValid = StrKey.isValidContract(trimmed);
    return isValid
      ? { isValid: true, sanitized: trimmed }
      : { isValid: false, error: "Invalid Soroban contract address format or checksum" };
  }

  // Muxed Account
  if (trimmed.startsWith("M")) {
    const isValid = StrKey.isValidMed25519PublicKey(trimmed);
    return isValid
      ? { isValid: true, sanitized: trimmed }
      : { isValid: false, error: "Invalid Stellar muxed account format or checksum" };
  }

  return {
    isValid: false,
    error: "Unrecognized Stellar address prefix. Must start with G, C, or M.",
  };
}
