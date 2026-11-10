import { SdkContractRegistry, KNOWN_CONTRACTS } from "../src/index";

describe("SdkContractRegistry Barrel Export", () => {
  it("should export SdkContractRegistry and KNOWN_CONTRACTS from SDK index", () => {
    expect(SdkContractRegistry).toBeDefined();
    expect(KNOWN_CONTRACTS).toBeDefined();
    expect(typeof SdkContractRegistry.get).toBe("function");
    expect(SdkContractRegistry.get("testnet").escrowContractId).toBe(
      KNOWN_CONTRACTS.testnet.escrowContractId
    );
  });
});
