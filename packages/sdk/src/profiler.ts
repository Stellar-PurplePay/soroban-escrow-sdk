import { ValidationError } from "./errors";

export interface ProfileSpan {
  name: string;
  startTime: number;
  endTime?: number;
  durationMs?: number;
  metadata?: Record<string, any>;
}

export interface ProfileReport {
  traceId: string;
  operation: string;
  totalDurationMs: number;
  spans: ProfileSpan[];
  timestamp: string;
}

/**
 * Performance timer profiler measuring granular spans across the Stellar transaction lifecycle.
 */
export class TransactionProfiler {
  public readonly traceId: string;
  public readonly operation: string;
  private spans: ProfileSpan[] = [];
  private t0: number;

  constructor(operation: string, traceId?: string) {
    if (!operation) {
      throw new ValidationError("Operation name is required for TransactionProfiler");
    }
    this.operation = operation;
    this.traceId = traceId || `trace-${Math.random().toString(36).substring(2, 10)}`;
    this.t0 = Date.now();
  }

  /**
   * Starts a named profiling span and returns a callback to stop it.
   */
  public startSpan(name: string, metadata?: Record<string, any>): () => void {
    if (!name) {
      throw new ValidationError("Span name is required");
    }
    const span: ProfileSpan = {
      name,
      startTime: Date.now(),
      metadata,
    };
    this.spans.push(span);

    return () => {
      span.endTime = Date.now();
      span.durationMs = Math.max(0, span.endTime - span.startTime);
    };
  }

  /**
   * Wraps an async function execution with span measurement.
   */
  public async measure<T>(
    name: string,
    fn: () => Promise<T>,
    metadata?: Record<string, any>
  ): Promise<T> {
    const stop = this.startSpan(name, metadata);
    try {
      return await fn();
    } finally {
      stop();
    }
  }

  /**
   * Generates a structured profile report.
   */
  public getReport(): ProfileReport {
    const totalDurationMs = Date.now() - this.t0;
    return {
      traceId: this.traceId,
      operation: this.operation,
      totalDurationMs,
      spans: [...this.spans],
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Generates a readable single-line summary of elapsed span times.
   */
  public formatSummary(): string {
    const report = this.getReport();
    const spanSummaries = report.spans.map((s) => `${s.name}: ${s.durationMs ?? 0}ms`);
    return `[PROFILER:${report.operation}] (total: ${report.totalDurationMs}ms) -> ${spanSummaries.join(", ")}`;
  }
}
