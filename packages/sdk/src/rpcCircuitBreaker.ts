export type CircuitState = "CLOSED" | "OPEN" | "HALF_OPEN";

export interface StateTransition {
  from: CircuitState;
  to: CircuitState;
  timestamp: number;
  reason?: string;
}

export interface CircuitBreakerReport {
  state: CircuitState;
  totalExecutions: number;
  totalFailures: number;
  totalShortCircuited: number;
  failureRatePercentage: number;
  lastFailureTime: number | null;
  lastStateChangeTime: number;
  uptimeSeconds: number;
  stateTransitions: StateTransition[];
}

export interface CircuitBreakerOptions {
  failureThreshold?: number;
  resetTimeoutMs?: number;
  halfOpenSuccessThreshold?: number;
  onStateChange?: (transition: StateTransition) => void;
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
  private lastStateChangeTime: number = Date.now();
  private readonly initTime: number = Date.now();

  private totalExecutions: number = 0;
  private totalFailures: number = 0;
  private totalShortCircuited: number = 0;
  private readonly transitions: StateTransition[] = [];

  private readonly failureThreshold: number;
  private readonly resetTimeoutMs: number;
  private readonly halfOpenSuccessThreshold: number;
  private readonly onStateChange?: (transition: StateTransition) => void;

  constructor(options: CircuitBreakerOptions = {}) {
    this.failureThreshold = options.failureThreshold ?? 5;
    this.resetTimeoutMs = options.resetTimeoutMs ?? 10_000;
    this.halfOpenSuccessThreshold = options.halfOpenSuccessThreshold ?? 2;
    this.onStateChange = options.onStateChange;
  }

  private transitionTo(newState: CircuitState, reason?: string): void {
    if (this.state === newState) return;
    const transition: StateTransition = {
      from: this.state,
      to: newState,
      timestamp: Date.now(),
      reason,
    };
    this.state = newState;
    this.lastStateChangeTime = transition.timestamp;
    this.transitions.push(transition);
    if (this.transitions.length > 50) {
      this.transitions.shift();
    }
    if (this.onStateChange) {
      this.onStateChange(transition);
    }
  }

  public getState(): CircuitState {
    if (this.state === "OPEN" && this.lastFailureTime !== null) {
      if (Date.now() - this.lastFailureTime >= this.resetTimeoutMs) {
        this.transitionTo("HALF_OPEN", "Reset timeout elapsed");
        this.successCount = 0;
      }
    }
    return this.state;
  }

  public async execute<T>(action: () => Promise<T>): Promise<T> {
    const currentState = this.getState();

    if (currentState === "OPEN") {
      this.totalShortCircuited++;
      throw new CircuitBreakerOpenError();
    }

    this.totalExecutions++;
    try {
      const result = await action();
      this.recordSuccess();
      return result;
    } catch (err: any) {
      this.totalFailures++;
      this.recordFailure(err);
      throw err;
    }
  }

  public recordSuccess(): void {
    if (this.state === "HALF_OPEN") {
      this.successCount++;
      if (this.successCount >= this.halfOpenSuccessThreshold) {
        this.transitionTo("CLOSED", "Probes succeeded");
        this.reset();
      }
    } else if (this.state === "CLOSED") {
      this.failureCount = 0;
    }
  }

  public recordFailure(err?: Error): void {
    this.lastFailureTime = Date.now();

    if (this.state === "HALF_OPEN") {
      this.transitionTo("OPEN", `Probe failed: ${err?.message || "error"}`);
      this.failureCount = this.failureThreshold;
    } else if (this.state === "CLOSED") {
      this.failureCount++;
      if (this.failureCount >= this.failureThreshold) {
        this.transitionTo("OPEN", `Exceeded failure threshold (${this.failureThreshold})`);
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

  public getDetailedReport(): CircuitBreakerReport {
    const totalAttempts = this.totalExecutions;
    const failureRate =
      totalAttempts > 0 ? (this.totalFailures / totalAttempts) * 100 : 0;

    return {
      state: this.getState(),
      totalExecutions: this.totalExecutions,
      totalFailures: this.totalFailures,
      totalShortCircuited: this.totalShortCircuited,
      failureRatePercentage: Math.round(failureRate * 100) / 100,
      lastFailureTime: this.lastFailureTime,
      lastStateChangeTime: this.lastStateChangeTime,
      uptimeSeconds: Math.floor((Date.now() - this.initTime) / 1000),
      stateTransitions: [...this.transitions],
    };
  }
}
