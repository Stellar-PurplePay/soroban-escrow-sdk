import { TransactionProfiler } from "../src/profiler";
import { ValidationError } from "../src/errors";

describe("TransactionProfiler", () => {
  it("measures asynchronous spans using measure() helper", async () => {
    const profiler = new TransactionProfiler("release_milestone", "test-trace-1");

    const result = await profiler.measure(
      "simulation",
      async () => {
        await new Promise((r) => setTimeout(r, 20));
        return { cpu: 5000 };
      },
      { contract: "CDEMO" }
    );

    expect(result.cpu).toBe(5000);

    const report = profiler.getReport();
    expect(report.traceId).toBe("test-trace-1");
    expect(report.operation).toBe("release_milestone");
    expect(report.spans.length).toBe(1);
    expect(report.spans[0].name).toBe("simulation");
    expect(report.spans[0].durationMs).toBeGreaterThanOrEqual(15);
    expect(report.spans[0].metadata?.contract).toBe("CDEMO");
  });

  it("measures manual start/stop spans across multiple stages", () => {
    const profiler = new TransactionProfiler("create_escrow");

    const stopSim = profiler.startSpan("simulation");
    stopSim();

    const stopSign = profiler.startSpan("wallet_signature");
    stopSign();

    const report = profiler.getReport();
    expect(report.spans.length).toBe(2);
    expect(report.spans[0].name).toBe("simulation");
    expect(report.spans[1].name).toBe("wallet_signature");

    const summary = profiler.formatSummary();
    expect(summary).toContain("[PROFILER:create_escrow]");
    expect(summary).toContain("simulation:");
    expect(summary).toContain("wallet_signature:");
  });

  it("throws ValidationError for empty operation or span name", () => {
    expect(() => new TransactionProfiler("")).toThrow(ValidationError);

    const profiler = new TransactionProfiler("valid");
    expect(() => profiler.startSpan("")).toThrow(ValidationError);
  });
});
