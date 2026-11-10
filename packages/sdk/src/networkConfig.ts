/**
 * NetworkConfig
 * Multi-network configuration resolver for Stellar public network, testnet, and futurenet.
 */

export interface NetworkDetails {
  name: "testnet" | "public" | "futurenet";
  networkPassphrase: string;
  rpcUrl: string;
  horizonUrl: string;
  friendbotUrl: string | null;
}

export const NETWORKS: Record<"testnet" | "public" | "futurenet", NetworkDetails> = {
  testnet: {
    name: "testnet",
    networkPassphrase: "Test SDF Network ; September 2015",
    rpcUrl: "https://soroban-testnet.stellar.org",
    horizonUrl: "https://horizon-testnet.stellar.org",
    friendbotUrl: "https://friendbot.stellar.org",
  },
  public: {
    name: "public",
    networkPassphrase: "Public Global Stellar Network ; July 2015",
    rpcUrl: "https://soroban-rpc.mainnet.stellar.org",
    horizonUrl: "https://horizon.stellar.org",
    friendbotUrl: null,
  },
  futurenet: {
    name: "futurenet",
    networkPassphrase: "Test SDF Future Network ; October 2022",
    rpcUrl: "https://rpc-futurenet.stellar.org",
    horizonUrl: "https://horizon-futurenet.stellar.org",
    friendbotUrl: "https://friendbot-futurenet.stellar.org",
  },
};

export class NetworkConfig {
  static get(network: string = "testnet"): NetworkDetails {
    const key = network.toLowerCase().trim();
    if (key === "mainnet" || key === "public") {
      return NETWORKS.public;
    }
    if (key === "futurenet") {
      return NETWORKS.futurenet;
    }
    return NETWORKS.testnet;
  }

  static resolvePassphrase(network: string = "testnet"): string {
    return this.get(network).networkPassphrase;
  }

  static isTestnet(network: string = "testnet"): boolean {
    const details = this.get(network);
    return details.name === "testnet" || details.name === "futurenet";
  }
}
