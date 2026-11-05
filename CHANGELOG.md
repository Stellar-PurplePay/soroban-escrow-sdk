# Changelog

All notable changes to the Soroban Escrow SDK and smart contract will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.2.0] - 2026-11-05

### Security & Hardening (Audit Remediation PP-11 through PP-16)
- **On-Chain Fee Protection (PP-11)**:
  - Enforced a strict on-chain ceiling constant `MAX_FEE_BPS = 1_000` (10.00% maximum platform fee) in `contracts/escrow/src/lib.rs`.
  - Added `validate_fee` enforcement across both `initialize()` and `set_fee_bps()`.
  - Added typed contract error `EscrowError::InvalidFee = 16`.
- **Milestone Vector Boundaries & Gas Exhaustion Resistance (PP-14)**:
  - Implemented `MAX_MILESTONES = 25` to prevent unbounded memory growth and storage denial-of-service.
  - Added checked addition when summing milestone percentages to prevent arithmetic overflow.
  - Restricted milestone descriptions to a maximum of 256 bytes.
  - Enforced rejection of zero-percentage milestones with `EscrowError::InvalidMilestoneData = 18` and `EscrowError::InvalidMilestoneCount = 17`.
- **Zero-Loss Monetary Precision (PP-12)**:
  - Completely redesigned `toAtomicUnits()` in `packages/sdk/src/utils.ts`.
  - Rejects amounts containing fractional digits exceeding the token's decimal precision by throwing a clear `ValidationError` rather than silently truncating or rounding.
  - Strictly rejects negative amounts, zero values, and scientific notation (e.g. `1e-5`).
- **Cryptographic Stellar Address Checksum Validation (PP-13)**:
  - Replaced regular expression checks with `@stellar/stellar-sdk`'s `StrKey.isValidEd25519PublicKey(address)` and `StrKey.isValidContract(address)`.
  - Enforces base32 payload length verification and CRC16-XModem checksum validation on all client, freelancer, token, and arbitrator addresses.
- **Modern CI Toolchain & Strict Linting (PP-15)**:
  - Updated compilation target to `wasm32v1-none` in `.github/workflows/ci.yml`.
  - Integrated `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, unit/integration tests, and release WASM compilation in CI.
- **Configurable Fallbacks (PP-16)**:
  - Removed reliance on hardcoded addresses in examples, documenting mandatory environment variables (`SOROBAN_CONTRACT_ID`, `STELLAR_NETWORK_PASSPHRASE`).

---

## [0.1.0] - 2026-07-01

### Added
- **Soroban Escrow Smart Contract**:
  - Non-custodial escrow lifecycle (`create_escrow`, `release_milestone`, `refund_escrow`, `timeout_refund`).
  - Pro-rata milestone payments with automatic rounding dust allocation to the final milestone.
  - Multi-token support for SEP-41 standard tokens including XLM and USDC.
  - Cryptographic deliverable hash anchoring (`submit_deliverable`).
  - Dispute raising and arbitrator resolution hooks (`dispute_escrow`, `resolve_dispute`).
  - Automatic storage TTL extension on instance storage keys.
- **TypeScript Client SDK (`@stellar-purplepay/escrow-sdk`)**:
  - `EscrowClient` class providing high-level invocation preparation and transaction building.
  - Full TypeScript type definitions for escrow parameters, milestones, and status enums.
  - Utility helpers for atomic unit conversion and address formatting.
  - Comprehensive Jest test suite covering client instantiation and payload building.
- **CI/CD Pipeline**:
  - GitHub Actions workflow automating Rust contract testing and TypeScript SDK builds.

