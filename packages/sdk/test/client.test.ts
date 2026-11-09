import { EscrowClient } from "../src/client";
import { ValidationError } from "../src/errors";
import { validateMilestones, toAtomicUnits, fromAtomicUnits, isValidStellarAddress } from "../src/utils";

describe("EscrowClient", () => {
  const mockConfig = {
    rpcUrl: "https://soroban-testnet.stellar.org",
    networkPassphrase: "Test SDF Network ; September 2015",
    contractId: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
  };

  const validClientAddress = "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN7";
  const validFreelancerAddress = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
  const validArbitratorAddress = "GAPTL2OLC4YDSJJ6MGT3MMBWIYD4GSVKQFU26647IOMBEEXUURNUM32X";
  const validTokenAddress = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";

  let client: EscrowClient;

  beforeEach(() => {
    client = new EscrowClient(mockConfig);
  });

  describe("Configuration & Initialization", () => {
    it("should instantiate successfully with valid config", () => {
      expect(client.rpcUrl).toBe(mockConfig.rpcUrl);
      expect(client.networkPassphrase).toBe(mockConfig.networkPassphrase);
      expect(client.contractId).toBe(mockConfig.contractId);
    });

    it("should throw error if config parameters are missing", () => {
      expect(() => new EscrowClient({ ...mockConfig, rpcUrl: "" })).toThrow(ValidationError);
      expect(() => new EscrowClient({ ...mockConfig, contractId: "" })).toThrow(ValidationError);
    });
  });

  describe("prepareCreateEscrow", () => {
    it("should prepare valid invocation payload for single milestone escrow", () => {
      const futureTimeout = Math.floor(Date.now() / 1000) + 7200;
      const payload = client.prepareCreateEscrow({
        jobId: "task-001",
        client: validClientAddress,
        freelancer: validFreelancerAddress,
        tokenAddress: validTokenAddress,
        amount: 10000000n,
        timeoutTimestamp: futureTimeout,
        arbitratorAddress: validArbitratorAddress,
      });

      expect(payload.method).toBe("create_escrow");
      expect(payload.args.job_id).toBe("task-001");
      expect(payload.args.params.total_amount).toBe(10000000n);
      expect(payload.args.milestones).toHaveLength(0);
    });

    it("should prepare valid invocation with custom milestones totaling 100%", () => {
      const futureTimeout = Math.floor(Date.now() / 1000) + 7200;
      const payload = client.prepareCreateEscrow({
        jobId: "task-002",
        client: validClientAddress,
        freelancer: validFreelancerAddress,
        tokenAddress: validTokenAddress,
        amount: 25000000n,
        timeoutTimestamp: futureTimeout,
        arbitratorAddress: validArbitratorAddress,
        milestones: [
          { description: "Sprint 1", percentage: 50 },
          { description: "Sprint 2", percentage: 50 },
        ],
      });

      expect(payload.args.milestones).toHaveLength(2);
      expect(payload.args.milestones[0].percentage).toBe(50);
    });

    it("should throw error if milestones do not total 100%", () => {
      const futureTimeout = Math.floor(Date.now() / 1000) + 7200;
      expect(() => {
        client.prepareCreateEscrow({
          jobId: "task-003",
          client: validClientAddress,
          freelancer: validFreelancerAddress,
          tokenAddress: validTokenAddress,
          amount: 25000000n,
          timeoutTimestamp: futureTimeout,
          arbitratorAddress: validArbitratorAddress,
          milestones: [
            { description: "Sprint 1", percentage: 40 },
            { description: "Sprint 2", percentage: 40 },
          ],
        });
      }).toThrow(ValidationError);
    });

    it("should throw error if timeout is in the past", () => {
      const pastTimeout = Math.floor(Date.now() / 1000) - 100;
      expect(() => {
        client.prepareCreateEscrow({
          jobId: "task-004",
          client: validClientAddress,
          freelancer: validFreelancerAddress,
          tokenAddress: validTokenAddress,
          amount: 10000000n,
          timeoutTimestamp: pastTimeout,
          arbitratorAddress: validArbitratorAddress,
        });
      }).toThrow(ValidationError);
    });
  });

  describe("Deliverables and Milestones", () => {
    it("should prepare valid submit_deliverable payload with 32-byte hash", () => {
      const validHash = "a".repeat(64);
      const payload = client.prepareSubmitDeliverable("job-1", validFreelancerAddress, validHash);
      expect(payload.method).toBe("submit_deliverable");
      expect(payload.args.deliverable_hash).toBe(validHash);
    });

    it("should reject invalid deliverable hashes", () => {
      expect(() => {
        client.prepareSubmitDeliverable("job-1", validFreelancerAddress, "short-hash");
      }).toThrow(ValidationError);
    });

    it("should prepare valid submit_milestone_deliverable payload with milestoneId", () => {
      const validHash = "b".repeat(64);
      const payload = client.prepareSubmitMilestoneDeliverable("job-1", validFreelancerAddress, 2, validHash);
      expect(payload.method).toBe("submit_milestone_deliverable");
      expect(payload.args.job_id).toBe("job-1");
      expect(payload.args.milestone_id).toBe(2);
      expect(payload.args.deliverable_hash).toBe(validHash);
    });

    it("should reject invalid milestoneId in prepareSubmitMilestoneDeliverable", () => {
      const validHash = "c".repeat(64);
      expect(() => {
        client.prepareSubmitMilestoneDeliverable("job-1", validFreelancerAddress, 0, validHash);
      }).toThrow(ValidationError);
    });

    it("should prepare release_milestone payload", () => {
      const payload = client.prepareReleaseMilestone("job-1", validClientAddress, 1);
      expect(payload.method).toBe("release_milestone");
      expect(payload.args.milestone_id).toBe(1);
    });

    it("should prepare release_partial_milestone payload with Stroop conversion", () => {
      const payload = client.prepareReleasePartialMilestone(
        "job-1",
        validClientAddress,
        1,
        "25.5000000"
      );
      expect(payload.method).toBe("release_partial_milestone");
      expect(payload.args.job_id).toBe("job-1");
      expect(payload.args.client).toBe(validClientAddress);
      expect(payload.args.milestone_id).toBe(1);
      expect(payload.args.amount).toBe(255000000n);
    });

    it("should accept BigInt amount in prepareReleasePartialMilestone", () => {
      const payload = client.prepareReleasePartialMilestone(
        "job-1",
        validClientAddress,
        2,
        500000n
      );
      expect(payload.args.amount).toBe(500000n);
    });

    it("should reject negative amount in prepareReleasePartialMilestone", () => {
      expect(() => {
        client.prepareReleasePartialMilestone("job-1", validClientAddress, 1, "-10");
      }).toThrow();
    });

    it("should prepare set_milestone_deadline payload", () => {
      const payload = client.prepareSetMilestoneDeadline(
        "job-1",
        validClientAddress,
        1,
        1700000000
      );
      expect(payload.method).toBe("set_milestone_deadline");
      expect(payload.args.job_id).toBe("job-1");
      expect(payload.args.client).toBe(validClientAddress);
      expect(payload.args.milestone_id).toBe(1);
      expect(payload.args.deadline).toBe(1700000000n);
    });

    it("should prepare cancel_expired_milestone payload", () => {
      const payload = client.prepareCancelExpiredMilestone(
        "job-1",
        validClientAddress,
        2
      );
      expect(payload.method).toBe("cancel_expired_milestone");
      expect(payload.args.job_id).toBe("job-1");
      expect(payload.args.client).toBe(validClientAddress);
      expect(payload.args.milestone_id).toBe(2);
    });

    it("should prepare extend_milestone_deadline payload with valid parameters", () => {
      const payload = client.prepareExtendMilestoneDeadline(
        "job-1",
        validClientAddress,
        1,
        86400
      );
      expect(payload.method).toBe("extend_milestone_deadline");
      expect(payload.args.job_id).toBe("job-1");
      expect(payload.args.client).toBe(validClientAddress);
      expect(payload.args.milestone_id).toBe(1);
      expect(payload.args.extension_seconds).toBe(86400n);
    });

    it("should reject negative extensionSeconds in prepareExtendMilestoneDeadline", () => {
      expect(() => {
        client.prepareExtendMilestoneDeadline("job-1", validClientAddress, 1, -3600);
      }).toThrow(ValidationError);
    });

    it("should reject invalid milestoneId in prepareCancelExpiredMilestone", () => {
      expect(() => {
        client.prepareCancelExpiredMilestone("job-1", validClientAddress, -1);
      }).toThrow(ValidationError);
    });

    it("should prepare submit_dispute_evidence payload with valid 32-byte hash", () => {
      const validEvidenceHash = "d".repeat(64);
      const payload = client.prepareSubmitDisputeEvidence(
        "job-1",
        validClientAddress,
        validEvidenceHash
      );
      expect(payload.method).toBe("submit_dispute_evidence");
      expect(payload.args.caller).toBe(validClientAddress);
      expect(payload.args.evidence_hash).toBe(validEvidenceHash);

      expect(() =>
        client.prepareSubmitDisputeEvidence("job-1", validClientAddress, "invalid-hash")
      ).toThrow("evidenceHashHex must be a 32-byte hexadecimal string");
    });
  });

  describe("Utility functions", () => {
    it("should convert to and from atomic units correctly", () => {
      const atomic = toAtomicUnits("100.5", 7);
      expect(atomic).toBe(1005000000n);

      const human = fromAtomicUnits(1005000000n, 7);
      expect(human).toBe("100.5");
    });

    it("should strictly reject excess precision rather than truncating", () => {
      // 8 decimals when max allowed is 7 must throw
      expect(() => toAtomicUnits("1.12345678", 7)).toThrow(ValidationError);
      expect(() => toAtomicUnits("10.00000001", 7)).toThrow(/Excess decimal precision/);
    });

    it("should reject negative amounts, zero, and scientific notation", () => {
      expect(() => toAtomicUnits("-100", 7)).toThrow(ValidationError);
      expect(() => toAtomicUnits("0", 7)).toThrow(/Amount must be strictly positive/);
      expect(() => toAtomicUnits("1e-5", 7)).toThrow(ValidationError);
      expect(() => toAtomicUnits("abc", 7)).toThrow(ValidationError);
    });

    it("should correctly validate Stellar addresses with checksums", () => {
      expect(isValidStellarAddress(validClientAddress)).toBe(true);
      expect(isValidStellarAddress(validTokenAddress)).toBe(true);
      expect(isValidStellarAddress("invalid_address")).toBe(false);
      // Corrupted StrKey checksum must fail
      const corruptedAddress = validClientAddress.slice(0, -1) + "X";
      expect(isValidStellarAddress(corruptedAddress)).toBe(false);
    });

    it("should enforce milestone count and description boundaries", () => {
      const tooMany = Array.from({ length: 26 }, () => ({
        description: "Milestone",
        percentage: 3,
      }));
      expect(() => validateMilestones(tooMany)).toThrow(/Milestone count cannot exceed 25/);

      const emptyDesc = [{ description: "", percentage: 100 }];
      expect(() => validateMilestones(emptyDesc)).toThrow(/Milestone description cannot be empty/);
    });
  });

  describe("Circuit Breaker & Mutual Cancel Payloads", () => {
    it("should prepare valid vote_dispute payload in quorum panel", () => {
      const payload = client.prepareVoteDispute(
        "job-quorum-1",
        validArbitratorAddress,
        600000n,
        400000n
      );
      expect(payload.method).toBe("vote_dispute");
      expect(payload.args.job_id).toBe("job-quorum-1");
      expect(payload.args.client_award).toBe(600000n);
      expect(payload.args.freelancer_award).toBe(400000n);
      expect(() =>
        client.prepareVoteDispute("job", "bad-arb", 100n, 100n)
      ).toThrow(ValidationError);
    });

    it("should prepare valid pause invocation", () => {
      const payload = client.preparePause();
      expect(payload.method).toBe("pause");
      expect(payload.contractId).toBe(client.contractId);
    });

    it("should prepare valid unpause invocation", () => {
      const payload = client.prepareUnpause();
      expect(payload.method).toBe("unpause");
      expect(payload.contractId).toBe(client.contractId);
    });

    it("should prepare valid mutual cancellation payload", () => {
      const payload = client.prepareMutualCancel("job-mutual");
      expect(payload.method).toBe("mutual_cancel");
      expect(payload.args.job_id).toBe("job-mutual");
    });

    it("should prepare valid set_fee_bps payload within 1000 bps ceiling", () => {
      const payload = client.prepareSetFeeBps(300);
      expect(payload.method).toBe("set_fee_bps");
      expect(payload.args.fee_bps).toBe(300);
      expect(() => client.prepareSetFeeBps(1001)).toThrow(/feeBps must be an integer between 0 and 1000/);
    });

    it("should prepare valid set_treasury payload with validated address", () => {
      const payload = client.prepareSetTreasury(validClientAddress);
      expect(payload.method).toBe("set_treasury");
      expect(payload.args.new_treasury).toBe(validClientAddress);
      expect(() => client.prepareSetTreasury("bad-address")).toThrow(/Invalid treasury address/);
    });

    it("should prepare valid set_admin payload with validated address", () => {
      const payload = client.prepareSetAdmin(validFreelancerAddress);
      expect(payload.method).toBe("set_admin");
      expect(payload.args.new_admin).toBe(validFreelancerAddress);
      expect(() => client.prepareSetAdmin("bad-address")).toThrow(/Invalid new admin address/);
    });

    it("should prepare valid register_arbitrator payload with positive bond", () => {
      const payload = client.prepareRegisterArbitrator(
        validArbitratorAddress,
        validTokenAddress,
        50000000n
      );
      expect(payload.method).toBe("register_arbitrator");
      expect(payload.args.arbitrator).toBe(validArbitratorAddress);
      expect(payload.args.bond_amount).toBe(50000000n);

      expect(() =>
        client.prepareRegisterArbitrator(validArbitratorAddress, validTokenAddress, 0n)
      ).toThrow("Bond amount must be strictly positive");
    });

    it("should prepare valid slash_arbitrator_bond payload with admin auth", () => {
      const payload = client.prepareSlashArbitrator(
        validClientAddress,
        validArbitratorAddress,
        validFreelancerAddress,
        validTokenAddress,
        25000000n
      );
      expect(payload.method).toBe("slash_arbitrator_bond");
      expect(payload.args.admin).toBe(validClientAddress);
      expect(payload.args.slash_amount).toBe(25000000n);

      expect(() =>
        client.prepareSlashArbitrator(
          validClientAddress,
          validArbitratorAddress,
          validFreelancerAddress,
          validTokenAddress,
          -10n
        )
      ).toThrow("Slash amount must be strictly positive");
    });

    it("should estimate resources via parseResourceEstimate helper", () => {
      const estimate = client.estimateResources({
        minResourceFee: 5000,
        cost: { cpuInsns: 10000, memBytes: 20000 },
      });
      expect(estimate.minResourceFeeStroops).toBe(5000);
      expect(estimate.cpuInstructions).toBe(10000);
    });

    it("should invoke operation with retry logic", async () => {
      const op = jest.fn().mockResolvedValue("RETRY_OK");
      const res = await client.invokeWithRetry(op);
      expect(res).toBe("RETRY_OK");
      expect(op).toHaveBeenCalledTimes(1);
    });

    it("should forward signWithFreighter with configured network passphrase", async () => {
      const mockFreighter = {
        isConnected: jest.fn().mockResolvedValue(true),
        getUserInfo: jest.fn().mockResolvedValue({ publicKey: validClientAddress }),
        signTransaction: jest.fn().mockResolvedValue("SIGNED_XDR"),
      };

      const signed = await client.signWithFreighter("DUMMY_XDR", {}, mockFreighter as any);
      expect(signed).toBe("SIGNED_XDR");
      expect(mockFreighter.signTransaction).toHaveBeenCalledWith("DUMMY_XDR", {
        networkPassphrase: "Test SDF Network ; September 2015",
      });
    });

    it("should probe contract health and return positive report on valid RPC response", async () => {
      const mockFetcher = jest.fn().mockResolvedValue({
        result: { sequence: 987654 },
      });

      const report = await client.getContractHealth(mockFetcher);
      expect(report.isResponsive).toBe(true);
      expect(report.latestLedgerSequence).toBe(987654);
      expect(report.contractId).toBe(client.contractId);
    });

    it("should handle RPC failure in getContractHealth", async () => {
      const mockFetcher = jest.fn().mockRejectedValue(new Error("RPC Timeout"));

      const report = await client.getContractHealth(mockFetcher);
      expect(report.isResponsive).toBe(false);
      expect(report.error).toBe("RPC Timeout");
    });

    it("should partition batch payments through prepareBatchPayments", () => {
      const summary = client.prepareBatchPayments([
        { destination: validClientAddress, amount: "25.0000000" },
        { destination: validFreelancerAddress, amount: "75.0000000" },
      ]);

      expect(summary.totalPayments).toBe(2);
      expect(summary.totalOperations).toBe(2);
      expect(summary.totalAmountStroops).toBe(1000000000n);
      expect(summary.totalAmountXlm).toBe("100.0000000");
      expect(summary.chunks).toHaveLength(1);
    });
  });
});
