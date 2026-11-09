import {
  partitionPayments,
  MAX_OPERATIONS_PER_TX,
  DEFAULT_BASE_FEE_PER_OP_STROOPS,
  PaymentItem,
} from "../src/batchPayment";
import { ValidationError } from "../src/errors";

describe("partitionPayments", () => {
  const dummyRecipient1 = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
  const dummyRecipient2 = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";

  it("partitions a single payment into one chunk", () => {
    const payments: PaymentItem[] = [
      { destination: dummyRecipient1, amount: "10.5000000" },
    ];

    const summary = partitionPayments(payments);

    expect(summary.totalPayments).toBe(1);
    expect(summary.totalOperations).toBe(1);
    expect(summary.requiredChunks).toBe(1);
    expect(summary.totalAmountStroops).toBe(105000000n);
    expect(summary.totalAmountXlm).toBe("10.5000000");
    expect(summary.totalEstimatedFeeStroops).toBe(100n);

    const chunk = summary.chunks[0];
    expect(chunk.chunkIndex).toBe(0);
    expect(chunk.operationCount).toBe(1);
    expect(chunk.payments).toHaveLength(1);
    expect(chunk.payments[0].destination).toBe(dummyRecipient1);
    expect(chunk.payments[0].asset).toBe("native");
  });

  it("partitions 250 payments into chunks of 100", () => {
    const payments: PaymentItem[] = [];
    for (let i = 0; i < 250; i++) {
      payments.push({
        destination: i % 2 === 0 ? dummyRecipient1 : dummyRecipient2,
        amount: "1.0000000",
      });
    }

    const summary = partitionPayments(payments);

    expect(summary.totalPayments).toBe(250);
    expect(summary.requiredChunks).toBe(3);
    expect(summary.chunks[0].operationCount).toBe(100);
    expect(summary.chunks[1].operationCount).toBe(100);
    expect(summary.chunks[2].operationCount).toBe(50);
    expect(summary.totalAmountStroops).toBe(2500000000n);
    expect(summary.totalEstimatedFeeStroops).toBe(250n * DEFAULT_BASE_FEE_PER_OP_STROOPS);
  });

  it("supports custom chunk sizes", () => {
    const payments: PaymentItem[] = [
      { destination: dummyRecipient1, amount: "1.0" },
      { destination: dummyRecipient2, amount: "2.0" },
      { destination: dummyRecipient1, amount: "3.0" },
    ];

    const summary = partitionPayments(payments, { maxOpsPerChunk: 2 });
    expect(summary.requiredChunks).toBe(2);
    expect(summary.chunks[0].operationCount).toBe(2);
    expect(summary.chunks[1].operationCount).toBe(1);
  });

  it("supports BigInt amounts directly", () => {
    const payments: PaymentItem[] = [
      { destination: dummyRecipient1, amount: 50_000_000n },
    ];

    const summary = partitionPayments(payments);
    expect(summary.totalAmountStroops).toBe(50_000_000n);
    expect(summary.totalAmountXlm).toBe("5.0000000");
  });

  it("throws ValidationError for empty payments array", () => {
    expect(() => partitionPayments([])).toThrow(ValidationError);
  });

  it("throws ValidationError for invalid destination address", () => {
    expect(() =>
      partitionPayments([{ destination: "invalid_address", amount: "5" }])
    ).toThrow("invalid destination Stellar address");
  });

  it("throws ValidationError for zero or negative amount", () => {
    expect(() =>
      partitionPayments([{ destination: dummyRecipient1, amount: "0" }])
    ).toThrow("greater than zero");
  });

  it("throws ValidationError for chunk size exceeding MAX_OPERATIONS_PER_TX", () => {
    expect(() =>
      partitionPayments(
        [{ destination: dummyRecipient1, amount: "1" }],
        { maxOpsPerChunk: 101 }
      )
    ).toThrow(`maxOpsPerChunk must be between 1 and ${MAX_OPERATIONS_PER_TX}`);
  });
});
