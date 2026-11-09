export type NetworkStatusLevel = "OPTIMAL" | "MODERATE" | "CONGESTED";

export interface NetworkMetricsSnapshot {
  latencyMs: number;
  feeMultiplier: number;
  baseFeeStroops: number;
  recommendedFeeStroops: number;
  ledgerSequence?: number;
  status: NetworkStatusLevel;
  lastChecked: number;
  error?: string;
}

export interface NetworkMetricsOptions {
  rpcUrl: string;
  horizonUrl?: string;
  defaultBaseFee?: number;
}

/**
 * Probes and calculates Stellar Horizon & Soroban network performance metrics,
 * fee surges, and RPC latency.
 */
export class NetworkMetricsCollector {
  private rpcUrl: string;
  private horizonUrl: string;
  private defaultBaseFee: number;
  private intervalId: ReturnType<typeof setInterval> | null = null;

  constructor(options: NetworkMetricsOptions) {
    this.rpcUrl = options.rpcUrl;
    this.horizonUrl = options.horizonUrl || options.rpcUrl.replace("soroban", "horizon");
    this.defaultBaseFee = options.defaultBaseFee || 100;
  }

  public classifyCongestion(multiplier: number, latencyMs: number): NetworkStatusLevel {
    if (multiplier >= 2.5 || latencyMs >= 3500) {
      return "CONGESTED";
    }
    if (multiplier >= 1.2 || latencyMs >= 1500) {
      return "MODERATE";
    }
    return "OPTIMAL";
  }

  public async sampleMetrics(
    fetchFn?: (url: string, opts?: any) => Promise<any>
  ): Promise<NetworkMetricsSnapshot> {
    const doFetch = fetchFn || (typeof fetch !== "undefined" ? fetch : null);
    const start = Date.now();

    if (!doFetch) {
      // Fallback synthetic snapshot if fetch is unavailable
      return {
        latencyMs: 120,
        feeMultiplier: 1.0,
        baseFeeStroops: this.defaultBaseFee,
        recommendedFeeStroops: this.defaultBaseFee,
        status: "OPTIMAL",
        lastChecked: Date.now(),
      };
    }

    try {
      const response = await doFetch(this.rpcUrl, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          jsonrpc: "2.0",
          id: 1,
          method: "getLatestLedger",
        }),
      });

      const latencyMs = Date.now() - start;
      const data = await response.json();

      let ledgerSeq: number | undefined;
      if (data?.result?.sequence) {
        ledgerSeq = Number(data.result.sequence);
      }

      // Compute surge multiplier based on latency and reported state
      let feeMultiplier = 1.0;
      if (latencyMs > 3000) {
        feeMultiplier = 3.0;
      } else if (latencyMs > 1500) {
        feeMultiplier = 1.5;
      }

      const recommendedFeeStroops = Math.round(this.defaultBaseFee * feeMultiplier);
      const status = this.classifyCongestion(feeMultiplier, latencyMs);

      return {
        latencyMs,
        feeMultiplier,
        baseFeeStroops: this.defaultBaseFee,
        recommendedFeeStroops,
        ledgerSequence: ledgerSeq,
        status,
        lastChecked: Date.now(),
      };
    } catch (err: any) {
      const latencyMs = Date.now() - start;
      return {
        latencyMs,
        feeMultiplier: 2.0,
        baseFeeStroops: this.defaultBaseFee,
        recommendedFeeStroops: this.defaultBaseFee * 2,
        status: "MODERATE",
        lastChecked: Date.now(),
        error: err?.message || "Failed to reach Soroban RPC endpoint",
      };
    }
  }

  public startMonitoring(
    intervalMs: number,
    callback: (snapshot: NetworkMetricsSnapshot) => void,
    fetchFn?: (url: string, opts?: any) => Promise<any>
  ): void {
    this.stopMonitoring();
    this.sampleMetrics(fetchFn).then(callback);
    this.intervalId = setInterval(() => {
      this.sampleMetrics(fetchFn).then(callback);
    }, Math.max(intervalMs, 1000));
  }

  public stopMonitoring(): void {
    if (this.intervalId !== null) {
      clearInterval(this.intervalId);
      this.intervalId = null;
    }
  }

  public isMonitoring(): boolean {
    return this.intervalId !== null;
  }
}
