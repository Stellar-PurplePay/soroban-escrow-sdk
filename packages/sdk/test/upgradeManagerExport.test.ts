import * as Sdk from "../src/index";

describe("upgradeManager barrel exports", () => {
  it("exports validateWasmHash function", () => {
    expect(typeof Sdk.validateWasmHash).toBe("function");
  });

  it("exports computeWasmHash function", () => {
    expect(typeof Sdk.computeWasmHash).toBe("function");
  });

  it("exports calculateTimelockStatus function", () => {
    expect(typeof Sdk.calculateTimelockStatus).toBe("function");
  });

  it("exports evaluateGovernanceQuorum function", () => {
    expect(typeof Sdk.evaluateGovernanceQuorum).toBe("function");
  });

  it("exports UpgradeManager class", () => {
    expect(typeof Sdk.UpgradeManager).toBe("function");
    expect(typeof Sdk.UpgradeManager.prepareUpgrade).toBe("function");
  });
});
