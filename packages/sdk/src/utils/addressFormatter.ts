export interface FormatAddressOptions {
  startChars?: number;
  endChars?: number;
  delimiter?: string;
  hyphenate?: boolean;
}

export interface AccessibleAddress {
  displayText: string;
  ariaLabel: string;
}

/**
 * Formats a Stellar public key or contract address for compact UI rendering
 * while preserving key boundary identifiers and checksum tails.
 */
export function formatStellarAddress(
  address: string,
  options: FormatAddressOptions = {}
): string {
  if (!address || typeof address !== "string") {
    return "";
  }

  const trimmed = address.trim();
  const start = options.startChars ?? 6;
  const end = options.endChars ?? 6;
  const delimiter = options.delimiter ?? "...";

  if (trimmed.length <= start + end) {
    return trimmed;
  }

  let formatted = `${trimmed.slice(0, start)}${delimiter}${trimmed.slice(-end)}`;

  if (options.hyphenate) {
    const head = trimmed.slice(0, start).match(/.{1,4}/g)?.join("-") || trimmed.slice(0, start);
    const tail = trimmed.slice(-end).match(/.{1,4}/g)?.join("-") || trimmed.slice(-end);
    formatted = `${head}${delimiter}${tail}`;
  }

  return formatted;
}

/**
 * Produces an accessible representation of a Stellar address for assistive screen readers.
 */
export function toAccessibleAddress(
  address: string,
  options: FormatAddressOptions = {}
): AccessibleAddress {
  const displayText = formatStellarAddress(address, options);
  if (!address || typeof address !== "string") {
    return { displayText: "", ariaLabel: "Empty Stellar address" };
  }

  const trimmed = address.trim();
  const start = trimmed.slice(0, 4).split("").join(" ");
  const end = trimmed.slice(-4).split("").join(" ");

  const type = trimmed.startsWith("C")
    ? "Contract address"
    : trimmed.startsWith("M")
    ? "Muxed account"
    : "Stellar account";

  const ariaLabel = `${type} beginning with ${start}, ending with ${end}`;

  return {
    displayText,
    ariaLabel,
  };
}
