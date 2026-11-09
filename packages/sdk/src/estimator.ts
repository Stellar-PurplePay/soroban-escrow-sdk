import { ValidationError } from "./errors";

export interface ResourceCost {
  cpuInstructions: number;
  memoryBytes: number;
  minResourceFeeStroops: number;
  recommendedInclusionFeeStroops: number;
  totalEstimatedFeeStroops: number;
  totalEstimatedFeeXlm: string;
  isBudgetExceeded: boolean;
}

export interface ResourceBudgetLimits {
  maxCpuInstructions?: number;
  maxMemoryBytes?: number;
}

export const DEFAULT_MAX_CPU = 100_000_000;
export const DEFAULT_MAX_MEM = 40_960_000;
export const STROOPS_PER_XLM = 10_000_000;
export const DEFAULT_BASE_INCLUSION_FEE = 100;

/**
 * Parses Soroban simulation response to extract CPU instructions, memory footprint,
 * and calculate recommended inclusion and total resource fees.
 *
 * @param simulationResponse - The raw response or result from Soroban RPC `simulateTransaction`
 * @param options - Custom base fee or resource budget limits for alerting
 * @returns Type-safe ResourceCost estimation
 */
export function parseResourceEstimate(
  simulationResponse: any,
  options?: {
    baseInclusionFeeStroops?: number;
    budgetLimits?: ResourceBudgetLimits;
  }
): ResourceCost {
  if (!simulationResponse || typeof simulationResponse !== "object") {
    throw new ValidationError("Simulation response must be a non-null object");
  }

  const minResourceFeeStroops = Number(
    simulationResponse.minResourceFee ??
      simulationResponse.result?.minResourceFee ??
      0
  );

  let cpuInstructions = 0;
  let memoryBytes = 0;

  if (simulationResponse.cost) {
    cpuInstructions = Number(simulationResponse.cost.cpuInsns || 0);
    memoryBytes = Number(simulationResponse.cost.memBytes || 0);
  } else if (simulationResponse.result?.cost) {
    cpuInstructions = Number(simulationResponse.result.cost.cpuInsns || 0);
    memoryBytes = Number(simulationResponse.result.cost.memBytes || 0);
  }

  const baseInclusionFee =
    options?.baseInclusionFeeStroops ?? DEFAULT_BASE_INCLUSION_FEE;
  const surgeBuffer = Math.ceil(minResourceFeeStroops * 0.15);
  const recommendedInclusionFeeStroops = baseInclusionFee + surgeBuffer;
  const totalEstimatedFeeStroops =
    minResourceFeeStroops + recommendedInclusionFeeStroops;

  const maxCpu = options?.budgetLimits?.maxCpuInstructions ?? DEFAULT_MAX_CPU;
  const maxMem = options?.budgetLimits?.maxMemoryBytes ?? DEFAULT_MAX_MEM;
  const isBudgetExceeded =
    cpuInstructions > maxCpu || memoryBytes > maxMem;

  const totalEstimatedFeeXlm = (
    totalEstimatedFeeStroops / STROOPS_PER_XLM
  ).toFixed(7);

  return {
    cpuInstructions,
    memoryBytes,
    minResourceFeeStroops,
    recommendedInclusionFeeStroops,
    totalEstimatedFeeStroops,
    totalEstimatedFeeXlm,
    isBudgetExceeded,
  };
}
