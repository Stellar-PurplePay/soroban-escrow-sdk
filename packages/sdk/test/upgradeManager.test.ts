import {
  validateWasmHash,
  computeWasmHash,
  calculateTimelockStatus,
  evaluateGovernanceQuorum,
  UpgradeManager,
} from "../src/upgradeManager";
import { ValidationError } from "../src/errors";

describe("upgradeManager", () => {
  const validHash = "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90";

  describe("validateWasmHash", () => {
    it("accepts valid 64-character lowercase hex string", () => {
      expect(validateWasmHash(validHash)).toBe(validHash);
    });

    it("normalizes uppercase hex characters to lowercase", () => {
      const upper = validHash.toUpperCase();
      expect(validateWasmHash(upper)).toBe(validHash);
    });

    it("rejects invalid length or non-hex string", () => {
      expect(() => validateWasmHash("tooshort")).toThrow(ValidationError);
      expect(() => validateWasmHash(validHash + "aa")).toThrow(ValidationError);
      expect(() => validateWasmHash("z".repeat(64))).toThrow(ValidationError);
      expect(() => validateWasmHash("")).toThrow(ValidationError);
    });
  });

  describe("computeWasmHash", () => {
    it("computes SHA-256 hash of wasm binary correctly", () => {
      const sampleWasm = Buffer.from([0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]);
      const hash = computeWasmHash(sampleWasm);
      expect(hash).toMatch(/^[0-9a-f]{64}$/);
    });

    it("throws ValidationError for empty buffer", () => {
      expect(() => computeWasmHash(Buffer.alloc(0))).toThrow(ValidationError);
    });
  });

  describe("calculateTimelockStatus", () => {
    it("returns remaining seconds when lock has not matured", () => {
      const status = calculateTimelockStatus(2000, 1500);
      expect(status.isUnlockMature).toBe(false);
      expect(status.remainingSeconds).toBe(500);
    });

    it("returns mature when current timestamp equals or exceeds unlock", () => {
      const status1 = calculateTimelockStatus(2000, 2000);
      expect(status1.isUnlockMature).toBe(true);
      expect(status1.remainingSeconds).toBe(0);

      const status2 = calculateTimelockStatus(2000, 2500);
      expect(status2.isUnlockMature).toBe(true);
      expect(status2.remainingSeconds).toBe(0);
    });

    it("throws on NaN timestamp", () => {
      expect(() => calculateTimelockStatus(NaN)).toThrow(ValidationError);
    });
  });

  describe("evaluateGovernanceQuorum", () => {
    it("evaluates failed proposal when quorum is not met", () => {
      const res = evaluateGovernanceQuorum(300, 100, 1000);
      expect(res.quorumMet).toBe(false);
      expect(res.passed).toBe(false);
      expect(res.totalVotes).toBe(400n);
      expect(res.forPercentage).toBe(75);
    });

    it("evaluates failed proposal when quorum met but against > for", () => {
      const res = evaluateGovernanceQuorum(400, 800, 1000);
      expect(res.quorumMet).toBe(true);
      expect(res.passed).toBe(false);
      expect(res.totalVotes).toBe(1200n);
      expect(res.forPercentage).toBeCloseTo(33.33, 1);
    });

    it("evaluates passed proposal when quorum met and for > against", () => {
      const res = evaluateGovernanceQuorum(1200, 300, 1000);
      expect(res.quorumMet).toBe(true);
      expect(res.passed).toBe(true);
      expect(res.totalVotes).toBe(1500n);
      expect(res.forPercentage).toBe(80);
    });

    it("throws on negative vote counts", () => {
      expect(() => evaluateGovernanceQuorum(-1, 10, 100)).toThrow(ValidationError);
    });
  });

  describe("UpgradeManager.prepareUpgrade", () => {
    it("prepares staged upgrade info with valid delay", () => {
      const staged = UpgradeManager.prepareUpgrade(validHash, 86400, 1000);
      expect(staged.newWasmHash).toBe(validHash);
      expect(staged.delaySeconds).toBe(86400);
      expect(staged.unlockTimestamp).toBe(1000 + 86400);
      expect(staged.isUnlockMature).toBe(false);
      expect(staged.remainingSeconds).toBe(86400);
    });

    it("rejects delays under 3,600 seconds", () => {
      expect(() => UpgradeManager.prepareUpgrade(validHash, 1800)).toThrow(
        /cannot be less than 3,600 seconds/
      );
    });
  });
});
