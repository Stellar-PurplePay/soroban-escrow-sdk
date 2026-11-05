# 🗺️ Project Roadmap & Drips Wave Sprint Backlog

This backlog contains scoped, high-impact tasks designed for open-source contributors during **Stellar Drips Wave** sprint cycles. Each issue includes complexity points, target components, and explicit acceptance criteria.

---

## 🎯 Active Sprint Issues & Points

| ID | Issue Title | Component | Complexity | Points |
| :--- | :--- | :--- | :--- | :--- |
| **#1** | [SDK] Add Freighter wallet transaction signing helper | `packages/sdk` | 🟡 Intermediate | **150 pts** |
| **#2** | [Contract] Migrate event emission to Soroban `#[contractevent]` | `contracts/escrow` | 🟢 Beginner | **100 pts** |
| **#3** | [SDK] Add resource fee and CPU instruction estimator helper | `packages/sdk` | 🟡 Intermediate | **150 pts** |
| **#4** | [Contract] Add proptest property-based tests for milestone splits | `contracts/escrow` | 🔴 Advanced | **200 pts** |
| **#5** | [SDK] Implement auto-retry with backoff on Soroban RPC 429s | `packages/sdk` | 🟡 Intermediate | **150 pts** |
| **#6** | [Contract] Support multi-arbitrator 2-of-3 quorum voting | `contracts/escrow` | 🔴 Advanced | **200 pts** |
| **#7** | [Docs] Add step-by-step local Soroban sandbox tutorial | `docs/` | 🟢 Beginner | **100 pts** |
| **#8** | [SDK] Export lightweight React hook `useEscrowState` | `packages/sdk` | 🟡 Intermediate | **150 pts** |

---

### Detailed Task Specifications

#### 1. [SDK] Add Freighter Wallet Transaction Signing Helper (150 pts)
* **Scope**: Enhance `@stellar-purplepay/escrow-sdk` to integrate smoothly with the `@stellar/freighter-api` browser extension.
* **Requirements**:
  - Add helper function `signWithFreighter(xdr: string, networkPassphrase: string)`.
  - Handle user rejection, network mismatch (Testnet vs Mainnet), and extension unlocked state gracefully.
  - Export types for wallet connection status.

#### 2. [Contract] Migrate Event Emission to `#[contractevent]` (100 pts)
* **Scope**: Soroban SDK 27+ introduces the typed `#[contractevent]` macro.
* **Requirements**:
  - Replace legacy `env.events().publish(...)` in `contracts/escrow/src/lib.rs` with typed contract event structs (`EscrowCreatedEvent`, `MilestoneReleasedEvent`, `TimeoutRefundEvent`, `DisputeRaisedEvent`).
  - Update unit tests in `src/test.rs` to verify event log topics.

#### 3. [SDK] Add Resource Fee and CPU Instruction Estimator Helper (150 pts)
* **Scope**: Provide developers with pre-flight fee visibility before submitting transactions.
* **Requirements**:
  - Add `estimateEscrowResources(options: CreateEscrowOptions): Promise<ResourceCost>`.
  - Parse `simulateTransaction` response for CPU instructions, memory bytes, and recommended inclusion fees.

#### 4. [Contract] Add Property-Based Fuzz Tests with `proptest` (200 pts)
* **Scope**: Strengthen contract safety against arithmetic overflow and odd milestone divisions.
* **Requirements**:
  - Integrate `proptest` dev-dependency in `contracts/escrow/Cargo.toml`.
  - Write property tests verifying that for any arbitrary number of milestones summing to 100%, total paid out across all milestones + fees strictly equals the initial escrow deposit.

#### 5. [SDK] Implement Auto-Retry with Backoff on Soroban RPC 429s (150 pts)
* **Scope**: Ensure production resilience when RPC endpoints rate limit requests.
* **Requirements**:
  - Implement configurable retry loop with exponential jitter (max retries: 3, base delay: 500ms).
  - Handle transient RPC errors (`ECONNRESET`, `429 Too Many Requests`).

#### 6. [Contract] Support Multi-Arbitrator 2-of-3 Quorum Voting (200 pts)
* **Scope**: Expand dispute arbitration from a single arbitrator address to a multi-sig quorum.
* **Requirements**:
  - Allow an array of 3 arbitrator addresses in `CreateEscrowParams`.
  - Require at least 2 distinct arbitrator signatures to execute `resolve_dispute`.

---

## 🚀 How to Claim & Submit
1. Choose an unassigned issue from the list above.
2. Comment on the corresponding GitHub Issue to be assigned.
3. Fork the repository, create a branch (`feat/issue-number-description`), and write unit tests.
4. Ensure all CI tests pass (`cargo test` and `npm test`).
5. Submit your PR referencing the issue number. Once merged, points are credited via Drips Wave!
