import {
  sanitizeString,
  validateAndSanitizeMemo,
  verifyStellarKey,
} from "../src/utils/sanitizer";

describe("SDK Sanitizer & StrKey Verifier", () => {
  const validGAddress = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
  const validCAddress = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";
  const validMAddress =
    "MA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJVAAAAAAAAAAAAAJLK";

  describe("sanitizeString", () => {
    it("strips scripts, event handlers, and null bytes", () => {
      const malicious = "Job Milestone\0 <script>alert('xss')</script> <img src=x onerror=steal() />";
      const cleaned = sanitizeString(malicious);
      expect(cleaned).not.toContain("<script>");
      expect(cleaned).not.toContain("\0");
      expect(cleaned).not.toContain("onerror=");
      expect(cleaned).toBe("Job Milestone <img src=x steal() />");
    });
  });

  describe("validateAndSanitizeMemo", () => {
    it("accepts valid memo within 28 bytes", () => {
      const res = validateAndSanitizeMemo("Escrow payout #1234");
      expect(res.isValid).toBe(true);
      expect(res.sanitized).toBe("Escrow payout #1234");
    });

    it("rejects memo exceeding 28 bytes UTF-8", () => {
      const longMemo = "This memo is definitely way longer than twenty eight bytes limit";
      const res = validateAndSanitizeMemo(longMemo);
      expect(res.isValid).toBe(false);
      expect(res.error).toContain("exceeds Stellar maximum of 28 UTF-8 bytes");
    });
  });

  describe("verifyStellarKey", () => {
    it("validates legitimate Ed25519 public key (G...)", () => {
      const res = verifyStellarKey(validGAddress);
      expect(res.isValid).toBe(true);
      expect(res.sanitized).toBe(validGAddress);
    });

    it("validates legitimate Soroban contract address (C...)", () => {
      const res = verifyStellarKey(validCAddress);
      expect(res.isValid).toBe(true);
      expect(res.sanitized).toBe(validCAddress);
    });

    it("validates legitimate Muxed account address (M...)", () => {
      const res = verifyStellarKey(validMAddress);
      expect(res.isValid).toBe(true);
      expect(res.sanitized).toBe(validMAddress);
    });

    it("rejects invalid checksum and unknown prefixes", () => {
      // Corrupted checksum
      const corrupted = validGAddress.slice(0, -4) + "AAAA";
      const res1 = verifyStellarKey(corrupted);
      expect(res1.isValid).toBe(false);
      expect(res1.error).toContain("Invalid Stellar Ed25519 public key format or checksum");

      // Invalid prefix (e.g. Ethereum 0x or Bitcoin 1)
      const res2 = verifyStellarKey("0x71C7656EC7ab88b098defB751B7401B5f6d8976F");
      expect(res2.isValid).toBe(false);
      expect(res2.error).toContain("Unrecognized Stellar address prefix");
    });
  });
});
