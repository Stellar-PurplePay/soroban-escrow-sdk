export interface RpcRetryOptions {
  maxRetries?: number;
  initialDelayMs?: number;
  maxDelayMs?: number;
  factor?: number;
  jitter?: boolean;
  onRetry?: (error: any, attempt: number, delayMs: number) => void;
}

/**
 * Determines whether a given network or RPC error is transient and safe to retry.
 */
export function isRetryableRpcError(error: any): boolean {
  if (!error) return false;
  const status = error.status || error.statusCode || error.response?.status;
  if (status === 429 || status === 503 || status === 504 || status === 502) {
    return true;
  }
  const message = (error.message || String(error)).toLowerCase();
  const retryablePhrases = [
    "rate limit",
    "too many requests",
    "429",
    "econnreset",
    "etimedout",
    "network error",
    "temporary failure",
    "socket hang up",
  ];
  return retryablePhrases.some((phrase) => message.includes(phrase));
}

/**
 * Wraps an asynchronous Soroban RPC operation with exponential backoff and jitter.
 *
 * @param fn - The async RPC function to execute
 * @param options - Retry configuration parameters
 * @returns The resolved value of the wrapped function
 */
export async function withRpcRetry<T>(
  fn: () => Promise<T>,
  options?: RpcRetryOptions
): Promise<T> {
  const maxRetries = options?.maxRetries ?? 3;
  const initialDelay = options?.initialDelayMs ?? 100;
  const maxDelay = options?.maxDelayMs ?? 2000;
  const factor = options?.factor ?? 2;
  const useJitter = options?.jitter ?? true;

  let attempt = 0;
  while (true) {
    try {
      return await fn();
    } catch (err: any) {
      attempt += 1;
      if (attempt > maxRetries || !isRetryableRpcError(err)) {
        throw err;
      }

      let delay = initialDelay * Math.pow(factor, attempt - 1);
      if (useJitter) {
        delay = Math.floor(Math.random() * delay);
      }
      delay = Math.min(delay, maxDelay);

      if (options?.onRetry) {
        options.onRetry(err, attempt, delay);
      }

      await new Promise((resolve) => setTimeout(resolve, delay));
    }
  }
}
