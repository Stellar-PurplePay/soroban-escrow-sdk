export class EscrowSdkError extends Error {
  constructor(message: string, public readonly code?: string) {
    super(message);
    this.name = "EscrowSdkError";
  }
}

export class ValidationError extends EscrowSdkError {
  constructor(message: string) {
    super(message, "VALIDATION_ERROR");
    this.name = "ValidationError";
  }
}

export class SimulationFailedError extends EscrowSdkError {
  constructor(message: string, public readonly details?: any) {
    super(message, "SIMULATION_FAILED");
    this.name = "SimulationFailedError";
  }
}

export class TransactionExecutionError extends EscrowSdkError {
  constructor(message: string, public readonly txHash?: string) {
    super(message, "TRANSACTION_EXECUTION_ERROR");
    this.name = "TransactionExecutionError";
  }
}
