import * as Sdk from "../src/index";

describe("profiler barrel exports", () => {
  it("exports TransactionProfiler class", () => {
    expect(typeof Sdk.TransactionProfiler).toBe("function");
    const profiler = new Sdk.TransactionProfiler("test");
    expect(typeof profiler.startSpan).toBe("function");
    expect(typeof profiler.measure).toBe("function");
    expect(typeof profiler.getReport).toBe("function");
  });
});
