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
- **Arbitrator Staking & Slashing**: Decentralized bond registration, balance queries, and malicious arbitrator penalty slashing.
- **Dispute Evidence Hashing**: Tamper-proof 32-byte cryptographic evidence logging anchored directly to on-chain state.
- **Batch Payout Partitioning**: Automatic splitting of multi-recipient transactions conforming to Stellar's 100 operations/envelope ceiling with Stroop fee estimation.
- **Milestone Deadlines & Overdue Cancellation**: Enforceable milestone completion timestamps with client fund recovery on unfulfilled deadlines.
- **Incremental Partial Releases**: Safe pro-rata releases for intermediate deliverables without full milestone finalization.
- **Monotonic Settlement Audit Receipts**: Sequential on-chain receipt counter ensuring immutable audit logs across releases, dispute resolutions, and timeout refunds.
- **Multi-Network Explorer Utilities**: Canonical link builder and address/hash truncators supporting Testnet, Futurenet, and Mainnet/Public.

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

### 9. Arbitrator Staking Bond & Penalty Slashing

```typescript
// Register arbitrator with 10,000 Stroop bond deposit
const registerPayload = client.prepareRegisterArbitrator(
  arbitratorAddress,
  10_000n
);

// Admin slashes compromised or unresponsive arbitrator bond
const slashPayload = client.prepareSlashArbitrator(
  arbitratorAddress,
  5_000n,
  recipientAddress
);
```

### 10. Dispute Evidence Submission

```typescript
// Anchor 32-byte cryptographic evidence hash on-chain
const evidencePayload = client.prepareSubmitDisputeEvidence(
  "job-101",
  submitterAddress,
  "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
);
```

### 11. Batch Payment Partitioning (100 Ops Envelope Limit)

```typescript
import { partitionPayments } from "@stellar-purplepay/escrow-sdk";

const batch = client.prepareBatchPayments([
  { destination: freelancerA, amount: "100.0" },
  { destination: freelancerB, amount: "250.0" },
]);

console.log(`Total Operations: ${batch.totalOperations}`);
console.log(`Required Envelopes: ${batch.requiredChunks}`);
console.log(`Network Fee: ${batch.totalEstimatedFeeStroops} Stroops`);
```

### 12. Milestone Deadlines & Overdue Cancellation

```typescript
// Client sets target completion deadline timestamp (UNIX seconds)
const setDlPayload = client.prepareSetMilestoneDeadline(
  "job-101",
  clientAddress,
  1,
  1750000000
);

// If deadline passes without completion, client reclaims milestone funds
const cancelPayload = client.prepareCancelExpiredMilestone(
  "job-101",
  clientAddress,
  1
);
```

### 13. Milestone Deadline Extension Negotiation

```typescript
// Client extends unapproved milestone deadline by 3 days (259,200 seconds)
const extendPayload = client.prepareExtendMilestoneDeadline(
  "job-101",
  clientAddress,
  1,
  259_200
);
```

### 14. Partial Milestone Releases & Progress Tracking

```typescript
// Client releases partial payment (40 XLM out of 100 XLM milestone)
const partialPayload = client.prepareReleasePartialMilestone(
  "job-101",
  clientAddress,
  1,
  "40.0000000"
);

// Client-side helper calculating progress and completion status
const progress = client.calculateMilestoneProgress("100.0", "40.0");
console.log(`Completed: ${progress.percentageReleased}%`); // 40.0%
console.log(`Remaining: ${progress.remainingXlm} XLM`); // 60.0000000 XLM
console.log(`Is Complete: ${progress.isComplete}`); // false
```

### 15. Settlement Receipt Tracking & Audit Logs

```typescript
// Query latest sequential receipt ID issued by the smart contract
const receiptCounterPayload = client.prepareGetReceiptCounter();

// Parse and validate contract receipt emitted during settlement
const receipt = client.parseReceiptEvent({
  receiptId: 42n,
  jobId: "job-101",
  milestoneId: 2,
  recipient: freelancerAddress,
  amount: 250_000_000n, // 25.0 XLM in Stroops
  fee: 2_500_000n,       // 0.25 XLM fee
});

console.log(`Receipt #${receipt.receiptId}: Paid ${receipt.amountXlm} XLM to ${receipt.recipient}`);
```

### 16. Stellar Explorer Link Generation & Formatting

```typescript
import { buildStellarExpertUrl, truncateIdentifier } from "@stellar-purplepay/escrow-sdk";

// Generate testnet or public explorer URL
const txUrl = buildStellarExpertUrl("tx", "e3b0c442...52b855", { network: "testnet" });
console.log(txUrl); // https://stellar.expert/explorer/testnet/tx/e3b0c442...52b855

// Truncate public key or hash for clean presentation
const shortAddress = truncateIdentifier("GDQP2KPQGKIHYJGXNUIYOMHARUARCA7DJT5FO2FFOOKY3IF53G4J7734", 6, 6);
console.log(shortAddress); // "GDQP2K...4J7734"
```

---

## 📜 License

MIT License. Copyright (c) 2026 Stellar PurplePay Community.
