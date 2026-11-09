export type CircuitState = "CLOSED" | "OPEN" | "HALF_OPEN";

export interface CircuitBreakerOptions {
  failureThreshold?: number;
  resetTimeoutMs?: number;
  halfOpenSuccessThreshold?: number;
}

export class CircuitBreakerOpenError extends Error {
  constructor(message: string = "Circuit breaker is OPEN. Upstream RPC is currently unavailable.") {
    super(message);
    this.name = "CircuitBreakerOpenError";
  }
}

/**
 * Resilient Circuit Breaker state machine protecting against cascading RPC node outages.
 * Transitions:
 * - CLOSED -> OPEN when consecutive failures >= failureThreshold
 * - OPEN -> HALF_OPEN after resetTimeoutMs elapses
 * - HALF_OPEN -> CLOSED when consecutive trial successes >= halfOpenSuccessThreshold
 * - HALF_OPEN -> OPEN immediately on any failure
 */
export class RpcCircuitBreaker {
  private state: CircuitState = "CLOSED";
  private failureCount: number = 0;
  private successCount: number = 0;
  private lastFailureTime: number | null = null;

  private readonly failureThreshold: number;
  private readonly resetTimeoutMs: number;
  private readonly halfOpenSuccessThreshold: number;

  constructor(options: CircuitBreakerOptions = {}) {
    this.failureThreshold = options.failureThreshold ?? 5;
    this.resetTimeoutMs = options.resetTimeoutMs ?? 10_000;
    this.halfOpenSuccessThreshold = options.halfOpenSuccessThreshold ?? 2;
  }

  public getState(): CircuitState {
    if (this.state === "OPEN" && this.lastFailureTime !== null) {
      if (Date.now() - this.lastFailureTime >= this.resetTimeoutMs) {
        this.state = "HALF_OPEN";
        this.successCount = 0;
      }
    }
    return this.state;
  }

  public async execute<T>(action: () => Promise<T>): Promise<T> {
    const currentState = this.getState();

    if (currentState === "OPEN") {
      throw new CircuitBreakerOpenError();
    }

    try {
      const result = await action();
      this.recordSuccess();
      return result;
    } catch (err: any) {
      this.recordFailure(err);
      throw err;
    }
  }

  public recordSuccess(): void {
    if (this.state === "HALF_OPEN") {
      this.successCount++;
      if (this.successCount >= this.halfOpenSuccessThreshold) {
        this.reset();
      }
    } else if (this.state === "CLOSED") {
      this.failureCount = 0;
    }
  }

  public recordFailure(_err?: Error): void {
    this.lastFailureTime = Date.now();

    if (this.state === "HALF_OPEN") {
      this.state = "OPEN";
      this.failureCount = this.failureThreshold;
    } else if (this.state === "CLOSED") {
      this.failureCount++;
      if (this.failureCount >= this.failureThreshold) {
        this.state = "OPEN";
      }
    }
  }

  public reset(): void {
    this.state = "CLOSED";
    this.failureCount = 0;
    this.successCount = 0;
    this.lastFailureTime = null;
  }

  public getMetrics(): {
    state: CircuitState;
    failures: number;
    successes: number;
    lastFailureTime: number | null;
  } {
    return {
      state: this.getState(),
      failures: this.failureCount,
      successes: this.successCount,
      lastFailureTime: this.lastFailureTime,
    };
  }
}
