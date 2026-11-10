import { NetworkConfig, NETWORKS } from "../src/index";

describe("NetworkConfig Barrel Export", () => {
  it("should export NetworkConfig and NETWORKS from index", () => {
    expect(NetworkConfig).toBeDefined();
    expect(NETWORKS).toBeDefined();
    expect(typeof NetworkConfig.get).toBe("function");
    expect(NetworkConfig.get("testnet").name).toBe("testnet");
  });
});
