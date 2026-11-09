import {
  RpcCircuitBreaker,
  CircuitBreakerOpenError,
} from "../src/rpcCircuitBreaker";

describe("RpcCircuitBreaker", () => {
  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it("starts in CLOSED state and executes actions successfully", async () => {
    const cb = new RpcCircuitBreaker({ failureThreshold: 3 });
    expect(cb.getState()).toBe("CLOSED");

    const result = await cb.execute(async () => "ok");
    expect(result).toBe("ok");
    expect(cb.getState()).toBe("CLOSED");
  });

  it("trips to OPEN state when consecutive failures reach threshold", async () => {
    const cb = new RpcCircuitBreaker({ failureThreshold: 3, resetTimeoutMs: 5000 });

    for (let i = 0; i < 2; i++) {
      await expect(
        cb.execute(async () => {
          throw new Error("RPC 500");
        })
      ).rejects.toThrow("RPC 500");
      expect(cb.getState()).toBe("CLOSED");
    }

    // 3rd failure trips the breaker
    await expect(
      cb.execute(async () => {
        throw new Error("RPC 500");
      })
    ).rejects.toThrow("RPC 500");
    expect(cb.getState()).toBe("OPEN");
  });

  it("fast-fails calls with CircuitBreakerOpenError when OPEN", async () => {
    const cb = new RpcCircuitBreaker({ failureThreshold: 2, resetTimeoutMs: 5000 });
    const actionSpy = jest.fn();

    // Trigger 2 failures to open breaker
    for (let i = 0; i < 2; i++) {
      await expect(
        cb.execute(async () => {
          throw new Error("RPC timeout");
        })
      ).rejects.toThrow();
    }
    expect(cb.getState()).toBe("OPEN");

    // Action should not be called while OPEN
    await expect(cb.execute(actionSpy)).rejects.toThrow(CircuitBreakerOpenError);
    expect(actionSpy).not.toHaveBeenCalled();
  });

  it("transitions to HALF_OPEN after timeout and recovers to CLOSED after successful probes", async () => {
    const cb = new RpcCircuitBreaker({
      failureThreshold: 2,
      resetTimeoutMs: 5000,
      halfOpenSuccessThreshold: 2,
    });

    // Trip breaker
    for (let i = 0; i < 2; i++) {
      await expect(
        cb.execute(async () => {
          throw new Error("RPC failure");
        })
      ).rejects.toThrow();
    }
    expect(cb.getState()).toBe("OPEN");

    // Advance time past reset timeout
    jest.advanceTimersByTime(5001);
    expect(cb.getState()).toBe("HALF_OPEN");

    // 1st probe success
    await cb.execute(async () => "probe-1");
    expect(cb.getState()).toBe("HALF_OPEN");

    // 2nd probe success restores CLOSED state
    await cb.execute(async () => "probe-2");
    expect(cb.getState()).toBe("CLOSED");
  });

  it("re-trips to OPEN immediately if a probe fails during HALF_OPEN", async () => {
    const cb = new RpcCircuitBreaker({
      failureThreshold: 2,
      resetTimeoutMs: 5000,
      halfOpenSuccessThreshold: 2,
    });

    for (let i = 0; i < 2; i++) {
      await expect(
        cb.execute(async () => {
          throw new Error("RPC error");
        })
      ).rejects.toThrow();
    }

    jest.advanceTimersByTime(5001);
    expect(cb.getState()).toBe("HALF_OPEN");

    // Probe failure returns directly to OPEN
    await expect(
      cb.execute(async () => {
        throw new Error("Probe failed");
      })
    ).rejects.toThrow("Probe failed");

    expect(cb.getState()).toBe("OPEN");
  });

  it("provides metrics and allows manual reset", () => {
    const cb = new RpcCircuitBreaker({ failureThreshold: 3 });
    cb.recordFailure(new Error("Test error"));
    const metrics = cb.getMetrics();

    expect(metrics.state).toBe("CLOSED");
    expect(metrics.failures).toBe(1);
    expect(metrics.lastFailureTime).not.toBeNull();

    cb.reset();
    const cleanMetrics = cb.getMetrics();
    expect(cleanMetrics.failures).toBe(0);
    expect(cleanMetrics.lastFailureTime).toBeNull();
  });

  it("fires onStateChange callback and tracks short-circuited calls in detailed report", async () => {
    const onStateChange = jest.fn();
    const cb = new RpcCircuitBreaker({
      failureThreshold: 2,
      resetTimeoutMs: 3000,
      onStateChange,
    });

    // Execute 1 success
    await cb.execute(async () => "success");

    // Execute 2 failures -> trips breaker
    for (let i = 0; i < 2; i++) {
      await expect(
        cb.execute(async () => {
          throw new Error("RPC error");
        })
      ).rejects.toThrow();
    }

    expect(onStateChange).toHaveBeenCalledWith(
      expect.objectContaining({ from: "CLOSED", to: "OPEN" })
    );

    // Call while OPEN -> short-circuits
    await expect(cb.execute(async () => "blocked")).rejects.toThrow(CircuitBreakerOpenError);

    const report = cb.getDetailedReport();
    expect(report.state).toBe("OPEN");
    expect(report.totalExecutions).toBe(3); // 1 success + 2 attempts
    expect(report.totalFailures).toBe(2);
    expect(report.totalShortCircuited).toBe(1);
    expect(report.failureRatePercentage).toBe(66.67);
    expect(report.stateTransitions).toHaveLength(1);
  });
});
