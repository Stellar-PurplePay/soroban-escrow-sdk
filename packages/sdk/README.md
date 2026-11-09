# 📦 @stellar-purplepay/escrow-sdk

> **Production TypeScript SDK for interacting with the Stellar Soroban milestone escrow smart contracts.**

[![Version](https://img.shields.io/badge/version-0.1.0-blue.svg)](package.json)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](../../LICENSE)
[![Stellar Soroban](https://img.shields.io/badge/Stellar-Soroban%20v27-08B5E5.svg?logo=stellar)](https://stellar.org)

---

## 🚀 Features

- **Freighter Wallet Integration**: Direct transaction signing with active network passphrase mismatch detection.
- **RPC Resilience**: Automatic exponential backoff and jitter on HTTP 429 rate limits and 5xx transient node failures.
- **Resource & Fee Estimation**: Simulation footprint analyzer parsing Soroban CPU instructions, memory bytes, and minimum resource fee in Stroops.
- **Multi-Arbitrator Quorum Voting**: Methods for submitting decentralized 2-of-3 arbitrator panel votes (`ClientFavor`, `FreelancerFavor`, `SplitHalf`).
- **Reactive State Subscription**: Real-time contract state tracking with poll interval management, status callbacks, and error reporting.
- **Emergency Circuit Breaker**: Transaction preparation for contract pause, unpause, and mutual cancellation with dual-party authorization.
- **Cryptographic Deliverable Proofs**: Client-side SHA-256 deliverable payload hashing with timestamped salt verification and granular per-milestone submissions.
- **Stroop Precision Math**: Exact BigInt Stroop arithmetic preventing IEEE-754 floating-point inaccuracies.
- **Contract & RPC Health Diagnostics**: Automated node pinging, ledger sequence tracking, and paused status reporting.

---

## 📦 Installation

```bash
npm install @stellar-purplepay/escrow-sdk @stellar/stellar-sdk
```

---

## 🛠️ Usage Guide

### 1. Initializing the Client

```typescript
import { EscrowClient, NetworkConfig } from "@stellar-purplepay/escrow-sdk";

const network: NetworkConfig = {
  rpcUrl: "https://soroban-testnet.stellar.org",
  networkPassphrase: "Test SDF Network ; September 2015",
  contractId: "CA5DEQZ64FKZL7KVWOEKXWJVZJXP4JKEQUAO7GFNYZYQRKI6N5CWEDU3",
};

const client = new EscrowClient(network);
```

### 2. Transaction Simulation & Resource Estimation

```typescript
// Simulate transaction to inspect CPU and resource fee before signing
const simResponse = await client.simulateTransaction(tx);
const estimate = client.estimateCost(simResponse);

console.log(`CPU Instructions: ${estimate.cpuInstructions}`);
console.log(`Memory Usage: ${estimate.memoryBytes} bytes`);
console.log(`Required Fee: ${estimate.minResourceFeeXlm} XLM`);
```

### 3. Signing with Freighter Wallet

```typescript
// Validates user wallet is connected to the matching network
const signedTxXdr = await client.signWithFreighter(tx.toXDR());
```

### 4. Quorum Dispute Resolution

```typescript
// Arbitrator submits vote on disputed escrow
const voteTx = client.prepareVoteDispute({
  jobId: "job-101",
  arbitrator: "GBWFZHBYEKF5MR2MHW7JAZX6ZRI2PKKBL6IALEMJSBMU4JMDSIB34KP5",
  vote: "FreelancerFavor", // "ClientFavor" | "FreelancerFavor" | "SplitHalf"
});
```

### 5. Reactive State Subscriptions

```typescript
const stateManager = client.subscribeEscrowState("job-101", {
  pollIntervalMs: 4000,
  onUpdate: (state) => {
    console.log(`Escrow status updated: ${state.status}`);
  },
  onError: (err) => {
    console.error(`Subscription poll error:`, err);
  },
});

// Unsubscribe when component unmounts
stateManager.stop();
```

### 6. Submitting Milestone Deliverable Proofs

```typescript
// Freelancer submits cryptographic proof for Milestone #2
const deliverablePayload = client.prepareSubmitMilestoneDeliverable(
  "job-101",
  freelancerAddress,
  2,
  "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
);
```

### 7. High-Precision Stroop Conversions

```typescript
import { toStroops, fromStroops } from "@stellar-purplepay/escrow-sdk";

const stroops = toStroops("15.5000000"); // 155,000,000n
const xlmString = fromStroops(155_000_000n); // "15.5000000"
```

### 8. Probing Node & Contract Health

```typescript
const health = await client.getContractHealth();
console.log(`Responsive: ${health.isResponsive}, Latency: ${health.rpcLatencyMs}ms, Ledger: #${health.latestLedgerSequence}`);
```

---

## 📜 License

MIT License. Copyright (c) 2026 Stellar PurplePay Community.
