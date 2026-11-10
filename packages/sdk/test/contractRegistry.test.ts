import { SdkContractRegistry, KNOWN_CONTRACTS } from "../src/contractRegistry";

describe("SdkContractRegistry Utility", () => {
  it("should resolve default testnet contract configuration", () => {
    const contracts = SdkContractRegistry.get("testnet");
    expect(contracts.escrowContractId).toBe(
      KNOWN_CONTRACTS.testnet.escrowContractId
    );
    expect(contracts.usdcTokenId).toBe(KNOWN_CONTRACTS.testnet.usdcTokenId);

    expect(SdkContractRegistry.getEscrowContractId("testnet")).toBe(
      KNOWN_CONTRACTS.testnet.escrowContractId
    );
  });

  it("should resolve public mainnet contract configuration", () => {
    const contracts = SdkContractRegistry.get("mainnet");
    expect(contracts.escrowContractId).toBe(
      KNOWN_CONTRACTS.public.escrowContractId
    );
    expect(SdkContractRegistry.getUsdcTokenId("public")).toBe(
      KNOWN_CONTRACTS.public.usdcTokenId
    );
  });
});
