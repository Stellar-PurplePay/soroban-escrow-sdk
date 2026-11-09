import { createDeliverableHash, verifyDeliverableHash } from "../src/hasher";
import { ValidationError } from "../src/errors";

describe("hasher", () => {
  it("computes standard SHA-256 hex string from string input", () => {
    const hash = createDeliverableHash("final-deliverable-v1.zip");
    expect(hash).toHaveLength(64);
    expect(/^[0-9a-f]{64}$/.test(hash)).toBe(true);
  });

  it("computes matching hash from Uint8Array input", () => {
    const text = "final-deliverable-v1.zip";
    const buffer = Buffer.from(text, "utf8");
    const hashStr = createDeliverableHash(text);
    const hashBuf = createDeliverableHash(buffer);
    expect(hashStr).toBe(hashBuf);
  });

  it("verifies matching hash correctly", () => {
    const content = "my milestone code commit";
    const expected = createDeliverableHash(content);
    expect(verifyDeliverableHash(content, expected)).toBe(true);
    expect(verifyDeliverableHash("tampered content", expected)).toBe(false);
  });

  it("throws ValidationError on empty or invalid input", () => {
    expect(() => createDeliverableHash("")).toThrow(ValidationError);
    expect(() => createDeliverableHash(null as any)).toThrow(ValidationError);
  });

  it("rejects invalid expectedHashHex format", () => {
    expect(() => verifyDeliverableHash("content", "short-hash")).toThrow(ValidationError);
  });
});

