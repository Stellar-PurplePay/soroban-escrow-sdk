export interface ContractEventMessage {
  id: string;
  topic: string[];
  data: any;
  ledger: number;
  ledgerClosedAt: string;
}

export interface ContractEventStreamerOptions {
  rpcUrl: string;
  contractId: string;
  startLedger?: number;
  pollIntervalMs?: number;
  initialBackoffMs?: number;
  maxBackoffMs?: number;
}

/**
 * Event streamer with exponential backoff reconnect for polling and dispatching
 * Soroban contract events.
 */
export class ContractEventStreamer {
  public readonly rpcUrl: string;
  public readonly contractId: string;
  private lastLedger: number;
  private pollIntervalMs: number;
  private initialBackoffMs: number;
  private maxBackoffMs: number;
  private currentBackoffMs: number;

  private isRunning = false;
  private pollTimeoutId: ReturnType<typeof setTimeout> | null = null;
  private listeners: Map<string, Set<(event: ContractEventMessage) => void>> = new Map();
  private wildcardListeners: Set<(event: ContractEventMessage) => void> = new Set();
  private reconnectAttempts = 0;

  constructor(options: ContractEventStreamerOptions) {
    this.rpcUrl = options.rpcUrl;
    this.contractId = options.contractId;
    this.lastLedger = options.startLedger || 0;
    this.pollIntervalMs = Math.max(500, options.pollIntervalMs || 2500);
    this.initialBackoffMs = options.initialBackoffMs || 1000;
    this.maxBackoffMs = options.maxBackoffMs || 30000;
    this.currentBackoffMs = this.initialBackoffMs;
  }

  public getIsRunning(): boolean {
    return this.isRunning;
  }

  public getLastLedger(): number {
    return this.lastLedger;
  }

  public getReconnectAttempts(): number {
    return this.reconnectAttempts;
  }

  public on(eventType: string, handler: (event: ContractEventMessage) => void): () => void {
    if (eventType === "*") {
      this.wildcardListeners.add(handler);
      return () => this.wildcardListeners.delete(handler);
    }

    if (!this.listeners.has(eventType)) {
      this.listeners.set(eventType, new Set());
    }
    const set = this.listeners.get(eventType)!;
    set.add(handler);

    return () => set.delete(handler);
  }

  public emit(eventType: string, event: ContractEventMessage): void {
    const handlers = this.listeners.get(eventType);
    if (handlers) {
      for (const handler of handlers) {
        handler(event);
      }
    }
    for (const wildcard of this.wildcardListeners) {
      wildcard(event);
    }
  }

  public async pollEvents(
    fetchFn?: (url: string, opts?: any) => Promise<any>
  ): Promise<number> {
    const doFetch = fetchFn || (typeof fetch !== "undefined" ? fetch : null);
    if (!doFetch) return 0;

    try {
      const body = {
        jsonrpc: "2.0",
        id: Date.now(),
        method: "getEvents",
        params: {
          startLedger: this.lastLedger > 0 ? this.lastLedger + 1 : undefined,
          filters: [
            {
              type: "contract",
              contractIds: [this.contractId],
            },
          ],
        },
      };

      const res = await doFetch(this.rpcUrl, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });

      const json = await res.json();
      const events: any[] = json?.result?.events || [];

      let count = 0;
      for (const raw of events) {
        const msg: ContractEventMessage = {
          id: String(raw.id || count),
          topic: Array.isArray(raw.topic) ? raw.topic : [],
          data: raw.value || raw.data,
          ledger: Number(raw.ledger || this.lastLedger),
          ledgerClosedAt: String(raw.ledgerClosedAt || new Date().toISOString()),
        };

        if (msg.ledger > this.lastLedger) {
          this.lastLedger = msg.ledger;
        }

        const eventType = msg.topic.length > 1 ? String(msg.topic[1]) : "unknown";
        this.emit(eventType, msg);
        count++;
      }

      // Success resets backoff
      this.currentBackoffMs = this.initialBackoffMs;
      this.reconnectAttempts = 0;
      return count;
    } catch (err) {
      this.reconnectAttempts++;
      this.currentBackoffMs = Math.min(
        this.maxBackoffMs,
        this.currentBackoffMs * 2
      );
      throw err;
    }
  }

  public start(fetchFn?: (url: string, opts?: any) => Promise<any>): void {
    if (this.isRunning) return;
    this.isRunning = true;

    const runPoll = async () => {
      if (!this.isRunning) return;
      try {
        await this.pollEvents(fetchFn);
        if (this.isRunning) {
          this.pollTimeoutId = setTimeout(runPoll, this.pollIntervalMs);
        }
      } catch {
        if (this.isRunning) {
          this.pollTimeoutId = setTimeout(runPoll, this.currentBackoffMs);
        }
      }
    };

    runPoll();
  }

  public stop(): void {
    this.isRunning = false;
    if (this.pollTimeoutId !== null) {
      clearTimeout(this.pollTimeoutId);
      this.pollTimeoutId = null;
    }
  }
}
