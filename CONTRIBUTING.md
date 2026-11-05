# Contributing to Soroban Escrow SDK

Thank you for your interest in contributing to **Soroban Escrow SDK**! We are building developer-first tooling for non-custodial milestone payments on the Stellar network.

---

## 🛠️ Development Setup

### Prerequisites
- **Rust & Cargo**: `>= 1.74`
- **Node.js**: `>= 20.x`
- **npm**: `>= 10.x`
- **Stellar CLI** (optional, for on-chain deployment): `>= 22.x`

### Clone & Install
```bash
git clone https://github.com/Stellar-PurplePay/soroban-escrow-sdk.git
cd soroban-escrow-sdk

# Install SDK dependencies
npm install --prefix packages/sdk
```

---

## 🧪 Running Tests

### 1. Smart Contract Tests (Rust / Cargo)
```bash
cd contracts/escrow
cargo test
```

### 2. TypeScript SDK Tests (Jest)
```bash
cd packages/sdk
npm test
```

---

## 📋 Pull Request Guidelines

1. **Focus on Single Issues**: Keep PRs scoped to the specific issue or feature described in [ROADMAP.md](ROADMAP.md).
2. **Include Tests**:
   - Contract changes must include corresponding unit tests in `contracts/escrow/src/test.rs`.
   - SDK additions must include unit tests in `packages/sdk/test/`.
3. **Commit Messages**: Follow standard conventional commits format:
   - `feat(contract): ...`
   - `fix(sdk): ...`
   - `docs: ...`
   - `test: ...`
4. **Code Quality**: Ensure zero compiler warnings and formatting checks pass before submitting.
