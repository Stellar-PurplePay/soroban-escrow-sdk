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

    it("should prepare release_milestone payload", () => {
      const payload = client.prepareReleaseMilestone("job-1", validClientAddress, 1);
      expect(payload.method).toBe("release_milestone");
      expect(payload.args.milestone_id).toBe(1);
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
  });
});
