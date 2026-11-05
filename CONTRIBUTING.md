# Contributing to Soroban Escrow SDK

Thank you for your interest in contributing to the **Soroban Escrow SDK**! We are building developer-first tooling for non-custodial milestone payments on the Stellar network.

---

## 🛠️ Development Setup

### Prerequisites
- **Rust & Cargo**: `>= 1.74`
- **WebAssembly Target**: `wasm32v1-none` (`rustup target add wasm32v1-none`)
- **Node.js**: `>= 20.x`
- **npm**: `>= 10.x`
- **Stellar CLI** (optional, for on-chain contract testing): `>= 22.x`

### Clone & Install
```bash
git clone https://github.com/Stellar-PurplePay/soroban-escrow-sdk.git
cd soroban-escrow-sdk

# Install SDK dependencies
cd packages/sdk
npm install
```

---

## 🧪 Testing & Code Quality Gates

Before opening a pull request, verify that all quality checks pass locally:

### 1. Smart Contract Formatting & Linting (Rust)
```bash
cd contracts/escrow

# 1. Format check
cargo fmt --check

# 2. Strict Clippy check (must have 0 warnings)
cargo clippy --all-targets -- -D warnings

# 3. Unit & Integration test suite
cargo test

# 4. Release WASM compilation
cargo build --target wasm32v1-none --release
```

### 2. TypeScript SDK Checks
```bash
cd packages/sdk

# 1. Run Jest test suite
npm test

# 2. Type-check & build
npm run build
```

---

## 📋 Pull Request Guidelines

1. **Focused Scopes**: Keep pull requests focused on a single feature, bugfix, or improvement.
2. **Comprehensive Tests**:
   - Contract changes must include test cases in `contracts/escrow/src/test.rs`.
   - SDK changes must include Jest test cases in `packages/sdk/test/`.
3. **Commit Messages**: Follow standard Conventional Commits:
   - `feat(contract): add milestone expiration hook`
   - `fix(sdk): validate StrKey checksums on arbitrator inputs`
   - `docs: update quickstart instructions in README`
4. **CI Health**: Ensure the automated GitHub Actions CI pipeline passes 100% green without warnings or failures.

---

## 📜 Code of Conduct & Security

Contributors are expected to adhere to our [Code of Conduct](CODE_OF_CONDUCT.md).  
To report security vulnerabilities, follow our [Security Policy](SECURITY.md).
