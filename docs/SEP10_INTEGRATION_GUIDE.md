# SEP-0010 Stellar Web Authentication & Escrow SDK Integration Guide

This guide details the cryptographic integration between Stellar Web Authentication (SEP-0010), Freighter wallet signing, and the Soroban Escrow SDK.

---

## 1. Architectural Overview

```
[User Browser / Wallet]
       │
       │ 1. GET /api/auth/sep10/challenge?account=G...
       ▼
[Backend Auth Service]
       │
       │ 2. Returns Challenge XDR signed by Server Keypair
       ▼
[User Browser / Wallet]
       │
       │ 3. Signs Challenge XDR using Freighter / Secret Key
       │
       │ 4. POST /api/auth/sep10/token { transaction: "<signedXDR>" }
       ▼
[Backend Auth Service]
       │
       │ 5. Validates nonces, timebounds, and signatures
       │ 6. Issues JWT bearer session token
       ▼
[Client Application]
       │
       │ 7. Calls Escrow SDK with JWT + Freighter signer
       ▼
[Soroban Smart Contracts]
```

---

## 2. SDK Authentication Flow Example

```typescript
import { Sep10Client } from "@stellar-purplepay/escrow-sdk";
import { signTransaction } from "@stellar/freighter-api";

// Initialize client with backend auth endpoint
const sep10 = new Sep10Client("https://api.testnet.stellar-purplepay.org");

async function authenticateUser(publicKey: string): Promise<string> {
  // 1. Fetch challenge XDR
  const { transaction, networkPassphrase } = await sep10.getChallenge(publicKey);

  // 2. Sign transaction via Freighter wallet extension
  const signedXdr = await signTransaction(transaction, { networkPassphrase });

  // 3. Exchange signed challenge for session JWT
  const { token } = await sep10.exchangeToken(signedXdr);

  // 4. Store token securely
  sessionStorage.setItem("purplepay_jwt", token);
  return token;
}
```

---

## 3. Escrow SDK Invocation with Wallet Authorization

```typescript
import { EscrowClient, NetworkConfig } from "@stellar-purplepay/escrow-sdk";

const network = NetworkConfig.get("testnet");
const escrow = new EscrowClient({
  contractId: "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
  rpcUrl: network.rpcUrl,
  networkPassphrase: network.networkPassphrase,
});

async function releaseMilestone(jobId: string, milestoneId: number, clientPublicKey: string) {
  return await escrow.releaseMilestone({
    jobId,
    milestoneId,
    client: clientPublicKey,
  });
}
```

---

## 4. Security Recommendations

1. **Strict Nonce Replay Prevention**: Server must verify sequence number `0` and unique cryptographic nonces.
2. **Short Timebounds**: Challenge transactions must expire within 300 seconds (5 minutes).
3. **Domain Verification**: Server must match `HOME_DOMAIN` with official web domain to prevent cross-site phishing.
