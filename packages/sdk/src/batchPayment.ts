/**
 * packages/sdk/src/batchPayment.ts
 *
 * Partitions high-volume payouts and milestone releases into chunks conforming to
 * Stellar's protocol ceiling of 100 operations per transaction envelope.
 */

import { ValidationError } from "./errors";
import { toStroops, fromStroops } from "./utils/stroopPrecision";

export const MAX_OPERATIONS_PER_TX = 100;
export const DEFAULT_BASE_FEE_PER_OP_STROOPS = 100n;

const STELLAR_ADDRESS_REGEX = /^[GM][A-Z0-9]{55,68}$/;

export interface PaymentItem {
  destination: string;
  amount: string | number | bigint;
  asset?: string;
  memo?: string;
}

export interface ValidatedPaymentItem {
  destination: string;
  amountStroops: bigint;
  amountXlm: string;
  asset: string;
  memo?: string;
}

export interface PaymentBatchChunk {
  chunkIndex: number;
  payments: ValidatedPaymentItem[];
  totalAmountStroops: bigint;
  totalAmountXlm: string;
  operationCount: number;
  estimatedFeeStroops: bigint;
}

export interface BatchPaymentSummary {
  totalAmountStroops: bigint;
  totalAmountXlm: string;
  totalPayments: number;
  totalOperations: number;
  requiredChunks: number;
  totalEstimatedFeeStroops: bigint;
  chunks: PaymentBatchChunk[];
}

export interface PartitionOptions {
  maxOpsPerChunk?: number;
  baseFeePerOpStroops?: bigint;
}

/**
 * Validates a list of payments and partitions them into transaction-sized chunks
 * conforming to the maximum operations allowed per Stellar transaction envelope.
 *
 * @param payments - Array of payment instructions
 * @param options - Configuration for chunk size and fee estimation
 * @returns BatchPaymentSummary with partitioned chunks and fee totals
 */
export function partitionPayments(
  payments: PaymentItem[],
  options: PartitionOptions = {}
): BatchPaymentSummary {
  if (!Array.isArray(payments) || payments.length === 0) {
    throw new ValidationError("Payments list must be a non-empty array");
  }

  const maxOps = options.maxOpsPerChunk ?? MAX_OPERATIONS_PER_TX;
  if (maxOps <= 0 || maxOps > MAX_OPERATIONS_PER_TX) {
    throw new ValidationError(
      `maxOpsPerChunk must be between 1 and ${MAX_OPERATIONS_PER_TX}`
    );
  }

  const baseFee = options.baseFeePerOpStroops ?? DEFAULT_BASE_FEE_PER_OP_STROOPS;
  if (baseFee <= 0n) {
    throw new ValidationError("baseFeePerOpStroops must be greater than zero");
  }

  const validated: ValidatedPaymentItem[] = payments.map((p, idx) => {
    if (!p.destination || !STELLAR_ADDRESS_REGEX.test(p.destination)) {
      throw new ValidationError(
        `Payment at index ${idx} has invalid destination Stellar address: "${p.destination}"`
      );
    }

    let stroops: bigint;
    if (typeof p.amount === "bigint") {
      if (p.amount <= 0n) {
        throw new ValidationError(`Payment amount at index ${idx} must be greater than zero`);
      }
      stroops = p.amount;
    } else {
      stroops = toStroops(p.amount);
      if (stroops <= 0n) {
        throw new ValidationError(`Payment amount at index ${idx} must be greater than zero`);
      }
    }

    return {
      destination: p.destination,
      amountStroops: stroops,
      amountXlm: fromStroops(stroops),
      asset: p.asset || "native",
      memo: p.memo,
    };
  });

  const chunks: PaymentBatchChunk[] = [];
  let currentPayments: ValidatedPaymentItem[] = [];
  let currentChunkStroops = 0n;

  for (let i = 0; i < validated.length; i++) {
    currentPayments.push(validated[i]);
    currentChunkStroops += validated[i].amountStroops;

    if (currentPayments.length === maxOps || i === validated.length - 1) {
      const opsCount = currentPayments.length;
      chunks.push({
        chunkIndex: chunks.length,
        payments: currentPayments,
        totalAmountStroops: currentChunkStroops,
        totalAmountXlm: fromStroops(currentChunkStroops),
        operationCount: opsCount,
        estimatedFeeStroops: baseFee * BigInt(opsCount),
      });
      currentPayments = [];
      currentChunkStroops = 0n;
    }
  }

  const totalAmountStroops = chunks.reduce((sum, c) => sum + c.totalAmountStroops, 0n);
  const totalOperations = chunks.reduce((sum, c) => sum + c.operationCount, 0);
  const totalEstimatedFeeStroops = chunks.reduce((sum, c) => sum + c.estimatedFeeStroops, 0n);

  return {
    totalAmountStroops,
    totalAmountXlm: fromStroops(totalAmountStroops),
    totalPayments: validated.length,
    totalOperations,
    requiredChunks: chunks.length,
    totalEstimatedFeeStroops,
    chunks,
  };
}
