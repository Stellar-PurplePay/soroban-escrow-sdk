import * as fs from "fs";
import * as path from "path";

describe("SEP10 Integration Guide Documentation Validation", () => {
  const docPath = path.resolve(__dirname, "../../../docs/SEP10_INTEGRATION_GUIDE.md");

  it("should locate SEP10_INTEGRATION_GUIDE.md on disk", () => {
    expect(fs.existsSync(docPath)).toBe(true);
    const content = fs.readFileSync(docPath, "utf8");
    expect(content).toContain("SEP-0010 Stellar Web Authentication");
  });

  it("should document code examples referencing Sep10Client and EscrowClient", () => {
    const content = fs.readFileSync(docPath, "utf8");
    expect(content).toContain("Sep10Client");
    expect(content).toContain("getChallenge");
    expect(content).toContain("exchangeToken");
    expect(content).toContain("releaseMilestone");
  });

  it("should document crucial security requirements for nonces, timebounds, and domains", () => {
    const content = fs.readFileSync(docPath, "utf8");
    expect(content).toContain("Strict Nonce Replay Prevention");
    expect(content).toContain("Short Timebounds");
    expect(content).toContain("Domain Verification");
  });
});
