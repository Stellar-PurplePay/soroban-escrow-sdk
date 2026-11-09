import {
  sanitizeString,
  validateAndSanitizeMemo,
  verifyStellarKey,
} from "../src";

describe("SDK Sanitizer Barrel Export", () => {
  it("exports sanitizeString from top-level SDK", () => {
    expect(typeof sanitizeString).toBe("function");
    expect(sanitizeString("Safe Text <script>evil()</script>")).toBe("Safe Text");
  });

  it("exports validateAndSanitizeMemo from top-level SDK", () => {
    expect(typeof validateAndSanitizeMemo).toBe("function");
    const valid = validateAndSanitizeMemo("Test memo");
    expect(valid.isValid).toBe(true);
    expect(valid.sanitized).toBe("Test memo");
  });

  it("exports verifyStellarKey from top-level SDK", () => {
    expect(typeof verifyStellarKey).toBe("function");
    const valid = verifyStellarKey("GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5");
    expect(valid.isValid).toBe(true);
  });
});
