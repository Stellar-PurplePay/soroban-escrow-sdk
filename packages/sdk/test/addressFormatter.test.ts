import {
  formatStellarAddress,
  toAccessibleAddress,
} from "../src/utils/addressFormatter";

describe("addressFormatter", () => {
  const gAddress = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
  const cAddress = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";

  describe("formatStellarAddress", () => {
    it("formats address with default 6...6 truncation", () => {
      const formatted = formatStellarAddress(gAddress);
      expect(formatted).toBe("GBBD47...LLFLA5");
    });

    it("supports custom startChars, endChars, and delimiter", () => {
      const formatted = formatStellarAddress(gAddress, {
        startChars: 4,
        endChars: 4,
        delimiter: " ~ ",
      });
      expect(formatted).toBe("GBBD ~ FLA5");
    });

    it("supports hyphenated chunking", () => {
      const formatted = formatStellarAddress(gAddress, {
        startChars: 8,
        endChars: 8,
        hyphenate: true,
      });
      expect(formatted).toBe("GBBD-47IF...3ZLL-FLA5");
    });

    it("returns original string if length is less than or equal to start + end", () => {
      expect(formatStellarAddress("SHORT")).toBe("SHORT");
      expect(formatStellarAddress("")).toBe("");
    });
  });

  describe("toAccessibleAddress", () => {
    it("generates screen reader announcement for Ed25519 public key", () => {
      const res = toAccessibleAddress(gAddress);
      expect(res.displayText).toBe("GBBD47...LLFLA5");
      expect(res.ariaLabel).toBe(
        "Stellar account beginning with G B B D, ending with F L A 5"
      );
    });

    it("generates screen reader announcement for Soroban contract address", () => {
      const res = toAccessibleAddress(cAddress);
      expect(res.displayText).toBe("CDLZFC...HGCYSC");
      expect(res.ariaLabel).toBe(
        "Contract address beginning with C D L Z, ending with C Y S C"
      );
    });
  });
});
