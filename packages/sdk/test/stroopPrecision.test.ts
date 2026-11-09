import {
  toStroops,
  fromStroops,
  STROOPS_PER_XLM_BIGINT,
} from "../src/utils/stroopPrecision";
import { ValidationError } from "../src/errors";

describe("stroopPrecision utility", () => {
  it("converts whole and decimal XLM strings to exact BigInt Stroops", () => {
    expect(toStroops("1")).toBe(10_000_000n);
    expect(toStroops("0.0000001")).toBe(1n);
    expect(toStroops("25.5")).toBe(255_000_000n);
    expect(toStroops("100.1234567")).toBe(1_001_234_567n);
  });

  it("converts BigInt Stroops back to 7-decimal formatted XLM strings", () => {
    expect(fromStroops(10_000_000n)).toBe("1.0000000");
    expect(fromStroops(1n)).toBe("0.0000001");
    expect(fromStroops(255_000_000n)).toBe("25.5000000");
    expect(fromStroops("10000000")).toBe("1.0000000");
  });

  it("rejects scientific notation", () => {
    expect(() => toStroops("1e7")).toThrow(ValidationError);
    expect(() => toStroops("1.5E-3")).toThrow(ValidationError);
  });

  it("rejects negative amounts", () => {
    expect(() => toStroops("-10")).toThrow(ValidationError);
    expect(() => fromStroops(-100n)).toThrow(ValidationError);
  });

  it("rejects excess decimal precision beyond 7 places", () => {
    expect(() => toStroops("1.12345678")).toThrow(ValidationError);
  });

  it("rejects non-numeric strings", () => {
    expect(() => toStroops("abc")).toThrow(ValidationError);
    expect(() => toStroops("")).toThrow(ValidationError);
  });

  it("verifies round-trip bijection invariance across randomized Stroop values", () => {
    // Generate 500 randomized Stroop amounts
    for (let i = 0; i < 500; i++) {
      // Scale from 1 Stroop to 10 billion Stroops (1,000 XLM)
      const wholePart = BigInt(Math.floor(Math.random() * 1_000_000));
      const fractionPart = BigInt(Math.floor(Math.random() * 10_000_000));
      const expectedStroops = wholePart * STROOPS_PER_XLM_BIGINT + fractionPart;

      const formattedXlm = fromStroops(expectedStroops);
      const convertedBack = toStroops(formattedXlm);

      expect(convertedBack).toBe(expectedStroops);

      const reFormatted = fromStroops(convertedBack);
      expect(reFormatted).toBe(formattedXlm);
    }
  });

  it("verifies additive linearity property without floating point drift", () => {
    // Proves (a + b) in Stroops matches BigInt addition perfectly
    const aStr = "123.4567890";
    const bStr = "876.5432110";

    const aStroops = toStroops(aStr);
    const bStroops = toStroops(bStr);

    const sumStroops = aStroops + bStroops;
    expect(sumStroops).toBe(10_000_000_000n); // exactly 1,000 XLM
    expect(fromStroops(sumStroops)).toBe("1000.0000000");
  });
});
