import * as Sdk from "../src/index";

describe("auditVerifier barrel exports", () => {
  it("exports computeAuditEntryHash function", () => {
    expect(typeof Sdk.computeAuditEntryHash).toBe("function");
  });

  it("exports verifyAuditChain function", () => {
    expect(typeof Sdk.verifyAuditChain).toBe("function");
  });

  it("exports AuditChainInspector class", () => {
    expect(typeof Sdk.AuditChainInspector).toBe("function");
  });

  it("runs an end-to-end audit verification through the barrel export", () => {
    const inspector = new Sdk.AuditChainInspector();
    inspector.append("escrow_created", { id: "job-100" });
    const result = Sdk.verifyAuditChain(inspector.getChain() as any);
    expect(result.isValid).toBe(true);
    expect(result.totalEntries).toBe(1);
  });
});
