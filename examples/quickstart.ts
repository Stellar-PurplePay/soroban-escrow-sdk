/**
 * Quickstart Example: Soroban Escrow SDK
 * Demonstrates creating a milestone escrow, starting work, and approving release.
 */

import { EscrowClient, toAtomicUnits } from "../packages/sdk/src";

async function main() {
  console.log("=== Stellar Soroban Escrow SDK Quickstart ===\n");

  const client = new EscrowClient({
    rpcUrl: "https://soroban-testnet.stellar.org",
    networkPassphrase: "Test SDF Network ; September 2015",
    contractId: "CA5DEQZ64FKZL7KVWOEKXWJVZJXP4JKEQUAO7GFNYZYQRKI6N5CWEDU3",
  });

  const clientAddress = "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN7";
  const freelancerAddress = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
  const arbitratorAddress = "GCKCWVE4EZP4SGF4YQ24WTYUGU6F4KDTQELFTX2NYW7K2G6KFFTY4VLL";
  const usdcTokenAddress = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";

  const jobId = "web3-frontend-audit";
  const budget = toAtomicUnits("500", 7); // 500 USDC
  const timeoutDeadline = Math.floor(Date.now() / 1000) + (14 * 24 * 3600); // 14 days

  // Step 1: Prepare Escrow with Milestones
  console.log(`1. Preparing Escrow Creation for Job [${jobId}]...`);
  const createTx = client.prepareCreateEscrow({
    jobId,
    client: clientAddress,
    freelancer: freelancerAddress,
    tokenAddress: usdcTokenAddress,
    amount: budget,
    timeoutTimestamp: timeoutDeadline,
    arbitratorAddress,
    milestones: [
      { description: "Milestone 1: Security Audit Report", percentage: 50 },
      { description: "Milestone 2: Fix Verification & Retest", percentage: 50 },
    ],
  });
  console.log("-> Ready to sign and submit:", createTx.method, "with budget", budget.toString());

  // Step 2: Freelancer starts work
  console.log(`\n2. Freelancer starts work...`);
  const startTx = client.prepareStartWork(jobId, freelancerAddress);
  console.log("-> Invocation method:", startTx.method);

  // Step 3: Freelancer submits deliverable proof
  console.log(`\n3. Submitting cryptographic deliverable hash...`);
  const sha256Proof = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
  const deliverableTx = client.prepareSubmitDeliverable(jobId, freelancerAddress, sha256Proof);
  console.log("-> Deliverable registered:", deliverableTx.args.deliverable_hash);

  // Step 4: Client approves Milestone 1
  console.log(`\n4. Client releases Milestone 1...`);
  const releaseTx = client.prepareReleaseMilestone(jobId, clientAddress, 1);
  console.log("-> Milestone 1 approved. Contract will transfer funds automatically.");
}

main().catch(console.error);
