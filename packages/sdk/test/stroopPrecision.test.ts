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
});
