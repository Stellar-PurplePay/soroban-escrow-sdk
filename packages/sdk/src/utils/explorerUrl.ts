export type StellarNetworkName = "testnet" | "futurenet" | "mainnet" | "public";
export type ExplorerEntity = "tx" | "account" | "contract" | "ledger";

export interface ExplorerUrlOptions {
  network?: StellarNetworkName;
  baseUrl?: string;
}

/**
 * Builds a StellarExpert URL for inspecting transactions, accounts, contracts, and ledgers.
 */
export function buildStellarExpertUrl(
  entity: ExplorerEntity,
  identifier: string | number,
  options: ExplorerUrlOptions = {}
): string {
  if (!identifier) return "";

  const rawNet = (options.network || "testnet").toLowerCase();
  const networkSegment = rawNet === "mainnet" || rawNet === "public" ? "public" : rawNet;
  const base = (options.baseUrl || "https://stellar.expert/explorer").replace(/\/+$/, "");

  let entityPath = entity as string;
  if (entity === "account") {
    entityPath = "account";
  } else if (entity === "tx") {
    entityPath = "tx";
  } else if (entity === "contract") {
    entityPath = "contract";
  } else if (entity === "ledger") {
    entityPath = "ledger";
  }

  return `${base}/${networkSegment}/${entityPath}/${encodeURIComponent(String(identifier).trim())}`;
}

/**
 * Truncates an on-chain Stellar identifier (address, contract, or tx hash) for clean UI presentation.
 */
export function truncateIdentifier(identifier: string, startChars: number = 6, endChars: number = 6): string {
  if (!identifier) return "";
  const cleaned = identifier.trim();
  if (cleaned.length <= startChars + endChars) return cleaned;
  return `${cleaned.slice(0, startChars)}...${cleaned.slice(-endChars)}`;
}
