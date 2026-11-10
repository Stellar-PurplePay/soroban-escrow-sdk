import { NetworkConfig, NETWORKS } from "../src/networkConfig";

describe("NetworkConfig Resolver", () => {
  it("should resolve testnet by default with official SDF testnet parameters", () => {
    const config = NetworkConfig.get("testnet");
    expect(config.name).toBe("testnet");
    expect(config.networkPassphrase).toBe(
      "Test SDF Network ; September 2015"
    );
    expect(config.rpcUrl).toContain("soroban-testnet.stellar.org");
    expect(config.friendbotUrl).toContain("friendbot.stellar.org");
    expect(NetworkConfig.isTestnet("testnet")).toBe(true);
  });

  it("should resolve public mainnet configuration with null friendbot", () => {
    const config = NetworkConfig.get("mainnet");
    expect(config.name).toBe("public");
    expect(config.networkPassphrase).toBe(
      "Public Global Stellar Network ; July 2015"
    );
    expect(config.friendbotUrl).toBeNull();
    expect(NetworkConfig.isTestnet("public")).toBe(false);
  });

  it("should resolve futurenet configuration", () => {
    const config = NetworkConfig.get("futurenet");
    expect(config.name).toBe("futurenet");
    expect(config.networkPassphrase).toContain("Future Network");
    expect(NetworkConfig.isTestnet("futurenet")).toBe(true);
  });

  it("should resolve correct passphrase via helper method", () => {
    expect(NetworkConfig.resolvePassphrase("testnet")).toBe(
      NETWORKS.testnet.networkPassphrase
    );
    expect(NetworkConfig.resolvePassphrase("public")).toBe(
      NETWORKS.public.networkPassphrase
    );
  });
});
