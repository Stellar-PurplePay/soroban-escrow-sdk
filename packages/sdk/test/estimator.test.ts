import { parseResourceEstimate } from "../src/estimator";
import { ValidationError } from "../src/errors";

describe("estimator", () => {
  it("rejects non-object or null input", () => {
    expect(() => parseResourceEstimate(null)).toThrow(ValidationError);
    expect(() => parseResourceEstimate("bad" as any)).toThrow(ValidationError);
  });

  it("parses valid simulation response with cost and minResourceFee", () => {
    const sim = {
      minResourceFee: 25000,
      cost: {
        cpuInsns: 1250000,
        memBytes: 450000,
      },
    };

    const estimate = parseResourceEstimate(sim);
    expect(estimate.cpuInstructions).toBe(1250000);
    expect(estimate.memoryBytes).toBe(450000);
    expect(estimate.minResourceFeeStroops).toBe(25000);
    // base (100) + surgeBuffer (25000 * 0.15 = 3750) = 3850
    expect(estimate.recommendedInclusionFeeStroops).toBe(3850);
    expect(estimate.totalEstimatedFeeStroops).toBe(28850);
    expect(estimate.totalEstimatedFeeXlm).toBe("0.0028850");
    expect(estimate.isBudgetExceeded).toBe(false);
  });

  it("detects budget exceeded when CPU or memory exceeds limit", () => {
    const sim = {
      minResourceFee: 1000,
      cost: {
        cpuInsns: 120_000_000, // exceeds 100m default
        memBytes: 1000,
      },
    };

    const estimate = parseResourceEstimate(sim);
    expect(estimate.isBudgetExceeded).toBe(true);
  });

  it("handles RPC nested result object", () => {
    const sim = {
      result: {
        minResourceFee: "15000",
        cost: {
          cpuInsns: "500000",
          memBytes: "200000",
        },
      },
    };

    const estimate = parseResourceEstimate(sim);
    expect(estimate.cpuInstructions).toBe(500000);
    expect(estimate.minResourceFeeStroops).toBe(15000);
  });
});
