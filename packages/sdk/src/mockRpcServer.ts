/**
 * MockSorobanRpcServer
 * Simulates Soroban RPC JSON-RPC 2.0 endpoints for deterministic unit testing,
 * load testing, and offline SDK development without requiring a live node.
 */

export interface MockRpcOptions {
  latestLedgerSequence?: number;
  protocolVersion?: number;
  simulateCost?: { cpuInsns: string; memBytes: string };
  defaultSendStatus?: "PENDING" | "ERROR" | "DUPLICATE";
  defaultTxStatus?: "SUCCESS" | "NOT_FOUND" | "FAILED";
}

export class MockSorobanRpcServer {
  private _latestLedger: number;
  private _protocolVersion: number;
  private _simulateCost: { cpuInsns: string; memBytes: string };
  private _defaultSendStatus: "PENDING" | "ERROR" | "DUPLICATE";
  private _defaultTxStatus: "SUCCESS" | "NOT_FOUND" | "FAILED";
  private _requestLog: Array<{ method: string; params: any; timestamp: string }> = [];

  constructor(options: MockRpcOptions = {}) {
    this._latestLedger = options.latestLedgerSequence || 100_000;
    this._protocolVersion = options.protocolVersion || 20;
    this._simulateCost = options.simulateCost || { cpuInsns: "50000", memBytes: "12000" };
    this._defaultSendStatus = options.defaultSendStatus || "PENDING";
    this._defaultTxStatus = options.defaultTxStatus || "SUCCESS";
  }

  getRequestLog() {
    return [...this._requestLog];
  }

  clearRequestLog() {
    this._requestLog = [];
  }

  async handleRequest(method: string, params: any = {}): Promise<any> {
    this._requestLog.push({ method, params, timestamp: new Date().toISOString() });

    switch (method) {
      case "getHealth":
        return { status: "healthy" };

      case "getLatestLedger":
        return {
          id: `ledger-hash-${this._latestLedger}`,
          sequence: this._latestLedger,
          protocolVersion: this._protocolVersion,
        };

      case "simulateTransaction":
        return {
          id: `sim-${Date.now()}`,
          latestLedger: this._latestLedger,
          minResourceFee: "150",
          cost: this._simulateCost,
          results: [
            {
              auth: [],
              xdr: "AAAAEAAAAAAAAAAAAAA=",
            },
          ],
        };

      case "sendTransaction":
        return {
          status: this._defaultSendStatus,
          hash: `tx-hash-${Date.now()}`,
          latestLedger: this._latestLedger,
          latestLedgerCloseTime: Math.floor(Date.now() / 1000).toString(),
        };

      case "getTransaction":
        return {
          status: this._defaultTxStatus,
          latestLedger: this._latestLedger,
          latestLedgerCloseTime: Math.floor(Date.now() / 1000).toString(),
          oldestLedger: this._latestLedger - 1000,
          oldestLedgerCloseTime: Math.floor(Date.now() / 1000 - 5000).toString(),
          envelopeXdr: "AAAA...",
          resultXdr: "AAAA...",
          resultMetaXdr: "AAAA...",
        };

      case "getEvents":
        return {
          latestLedger: this._latestLedger,
          events: [],
        };

      default:
        throw new Error(`Unsupported Mock Soroban RPC method: ${method}`);
    }
  }

  advanceLedger(steps: number = 1): number {
    this._latestLedger += steps;
    return this._latestLedger;
  }
}
