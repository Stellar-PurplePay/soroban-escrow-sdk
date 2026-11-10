/**
 * ContractRegistry
 * Client-side registry resolving deployed Soroban escrow contracts and token addresses.
 */

export interface NetworkContracts {
  escrowContractId: string;
  usdcTokenId: string;
}

export const KNOWN_CONTRACTS: Record<"testnet" | "public" | "futurenet", NetworkContracts> = {
  testnet: {
    escrowContractId: "CAQ3J7QGZGM56J5XF3RKV4Z3B7ZPV7VBNQ7QZGM56J5XF3RKV4Z3B7ZP",
    usdcTokenId: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
  },
  public: {
    escrowContractId: "CBM7QZGM56J5XF3RKV4Z3B7ZPV7VBNQ7QZGM56J5XF3RKV4Z3B7ZPV7V",
    usdcTokenId: "CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75",
  },
  futurenet: {
    escrowContractId: "CFUTURE7QZGM56J5XF3RKV4Z3B7ZPV7VBNQ7QZGM56J5XF3RKV4Z3B7Z",
    usdcTokenId: "CFUTUREUSDC7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSCFUT",
  },
};

export class SdkContractRegistry {
  static get(network: string = "testnet"): NetworkContracts {
    const key = (network || "testnet").toLowerCase().trim();
    if (key === "mainnet" || key === "public") {
      return KNOWN_CONTRACTS.public;
    }
    if (key === "futurenet") {
      return KNOWN_CONTRACTS.futurenet;
    }
    return KNOWN_CONTRACTS.testnet;
  }

  static getEscrowContractId(network: string = "testnet"): string {
    return this.get(network).escrowContractId;
  }

  static getUsdcTokenId(network: string = "testnet"): string {
    return this.get(network).usdcTokenId;
  }
}
