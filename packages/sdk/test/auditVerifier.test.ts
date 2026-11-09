import {
  computeAuditEntryHash,
  verifyAuditChain,
  AuditChainInspector,
  AuditEntry,
} from "../src/auditVerifier";
import { ValidationError } from "../src/errors";

describe("auditVerifier", () => {
  describe("computeAuditEntryHash", () => {
    it("computes deterministic SHA-256 hash for identical entries", () => {
      const entry = {
        index: 1,
        action: "escrow_created",
        timestamp: "2026-11-10T00:00:00Z",
        payload: { amount: 500, asset: "USDC" },
        previousHash: "0".repeat(64),
      };

      const hash1 = computeAuditEntryHash(entry);
      const hash2 = computeAuditEntryHash(entry);
      expect(hash1).toBe(hash2);
      expect(hash1).toMatch(/^[0-9a-f]{64}$/);
    });

    it("throws ValidationError when index or action is missing", () => {
      expect(() =>
        computeAuditEntryHash({
          index: undefined as any,
          action: "escrow_created",
          timestamp: "2026",
          payload: {},
          previousHash: "0".repeat(64),
        })
      ).toThrow(ValidationError);

      expect(() =>
        computeAuditEntryHash({
          index: 1,
          action: "",
          timestamp: "2026",
          payload: {},
          previousHash: "0".repeat(64),
        })
      ).toThrow(ValidationError);
    });
  });

  describe("AuditChainInspector & verifyAuditChain", () => {
    it("builds a valid tamper-evident chain and verifies successfully", () => {
      const inspector = new AuditChainInspector();
      const e1 = inspector.append("escrow_created", { id: "job-1", amount: 1000 }, 1000);
      const e2 = inspector.append("milestone_approved", { milestoneId: 1 }, 2000);
      const e3 = inspector.append("dispute_resolved", { clientPart: 500 }, 3000);

      expect(e1.index).toBe(1);
      expect(e2.index).toBe(2);
      expect(e3.index).toBe(3);

      expect(e2.previousHash).toBe(e1.entryHash);
      expect(e3.previousHash).toBe(e2.entryHash);

      const verification = inspector.verify();
      expect(verification.isValid).toBe(true);
      expect(verification.totalEntries).toBe(3);
      expect(verification.rootHash).toBe(e3.entryHash);
      expect(inspector.getRootHash()).toBe(e3.entryHash);
    });

    it("handles empty audit chains", () => {
      const result = verifyAuditChain([]);
      expect(result.isValid).toBe(true);
      expect(result.totalEntries).toBe(0);
    });

    it("detects tampered payload content within a chain", () => {
      const inspector = new AuditChainInspector();
      inspector.append("escrow_created", { amount: 1000 }, 1000);
      inspector.append("milestone_approved", { milestoneId: 1 }, 2000);

      const chain: AuditEntry[] = JSON.parse(JSON.stringify(inspector.getChain()));
      // Maliciously tamper with the payload of entry 1
      chain[0].payload = { amount: 99999999 };

      const result = verifyAuditChain(chain);
      expect(result.isValid).toBe(false);
      expect(result.brokenIndex).toBe(1);
      expect(result.error).toContain("Tampered hash at index 1");
    });

    it("detects broken link when previousHash is manipulated", () => {
      const inspector = new AuditChainInspector();
      inspector.append("event1", { data: 1 }, 1000);
      inspector.append("event2", { data: 2 }, 2000);

      const chain: AuditEntry[] = JSON.parse(JSON.stringify(inspector.getChain()));
      // Alter previousHash of entry 2 to point to fake parent
      chain[1].previousHash = "a".repeat(64);

      const result = verifyAuditChain(chain);
      expect(result.isValid).toBe(false);
      expect(result.brokenIndex).toBe(2);
      expect(result.error).toContain("Broken chain link at index 2");
    });

    it("detects non-sequential index jumping", () => {
      const inspector = new AuditChainInspector();
      const e1 = inspector.append("event1", { a: 1 }, 1000);
      const e2 = inspector.append("event2", { b: 2 }, 2000);

      const chain: AuditEntry[] = [
        e1,
        { ...e2, index: 5 }, // index skipped from 1 to 5
      ];

      const result = verifyAuditChain(chain);
      expect(result.isValid).toBe(false);
      expect(result.brokenIndex).toBe(5);
      expect(result.error).toContain("Non-sequential index");
    });

    it("validates expected root hash correctly and detects root hash mismatch", () => {
      const inspector = new AuditChainInspector();
      inspector.append("event1", { ok: true }, 1000);
      const expectedRoot = inspector.getRootHash()!;

      const success = inspector.verify(expectedRoot);
      expect(success.isValid).toBe(true);

      const failure = inspector.verify("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
      expect(failure.isValid).toBe(false);
      expect(failure.error).toContain("Root hash mismatch");
    });

    it("rejects non-array input with ValidationError", () => {
      expect(() => verifyAuditChain("not-an-array" as any)).toThrow(ValidationError);
    });
  });
});
