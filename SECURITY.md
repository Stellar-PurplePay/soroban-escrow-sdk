# Security Policy

## Reporting a Vulnerability

The **Soroban Escrow SDK** provides smart contract escrow and client tooling for on-chain fund custody on Stellar. We take security seriously and appreciate your help disclosing vulnerabilities responsibly.

### Contact & Reporting

Use **GitHub Private Vulnerability Reporting** at:

https://github.com/Stellar-PurplePay/soroban-escrow-sdk/security/advisories

If you cannot use GitHub's private reporting, email:  
**security@stellarpurplepay.dev**

### What to Include in Your Report

To help us investigate and triage quickly, please include:
- A descriptive summary of the vulnerability
- Exact steps to reproduce or a working proof-of-concept
- Affected component(s) (e.g. `contracts/escrow/src/lib.rs` or `packages/sdk/src/utils.ts`)
- The commit hash or release version evaluated
- Potential impact and suggested remediation (if known)

---

## Scope

The following components are **in scope** for security reviews:
- **Soroban Smart Contract**: `contracts/escrow/` — Rust contract logic, access control, fund transfers, rounding math, and milestone state transitions.
- **TypeScript SDK**: `packages/sdk/` — Invocation payload generation, argument serialization, precision conversion, and address checksum validation.
- **CI/CD Pipelines**: `.github/workflows/` — Automated build, linting, and testing pipelines.

---

## Out of Scope

- Theoretical vulnerabilities without demonstrable practical exploitability
- Vulnerabilities in third-party systems outside this repository (e.g. the core Stellar Consensus Protocol, third-party wallet extensions)
- Social engineering, phishing, or denial-of-service attacks against developer infrastructure

---

## Response SLA

| Phase | Target Timeline |
| :--- | :--- |
| **Initial Acknowledgment** | Within **48 hours** |
| **Triage & Severity Assessment** | Within **5 business days** |
| **Patch Development & Testing** | Within **14 business days** |
| **Public Disclosure & Release** | Coordinated with reporter upon deployment or within **90 days** |

We adhere to standard coordinated vulnerability disclosure best practices.
