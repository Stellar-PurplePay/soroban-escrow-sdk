import { withRpcRetry, isRetryableRpcError } from "../src/rpcRetry";

describe("rpcRetry", () => {
  it("correctly identifies retryable errors", () => {
    expect(isRetryableRpcError({ status: 429 })).toBe(true);
    expect(isRetryableRpcError({ statusCode: 503 })).toBe(true);
    expect(isRetryableRpcError(new Error("Rate limit exceeded"))).toBe(true);
    expect(isRetryableRpcError(new Error("read ECONNRESET"))).toBe(true);

    expect(isRetryableRpcError(new Error("Invalid transaction syntax"))).toBe(false);
    expect(isRetryableRpcError({ status: 400 })).toBe(false);
    expect(isRetryableRpcError(null)).toBe(false);
  });

  it("returns value immediately on success", async () => {
    const fn = jest.fn().mockResolvedValue("SUCCESS");
    const result = await withRpcRetry(fn);
    expect(result).toBe("SUCCESS");
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("retries on 429 and resolves when subsequent attempt succeeds", async () => {
    const error = new Error("Rate limit 429");
    const fn = jest
      .fn()
      .mockRejectedValueOnce(error)
      .mockRejectedValueOnce(error)
      .mockResolvedValueOnce("RECOVERED");

    const onRetry = jest.fn();
    const result = await withRpcRetry(fn, {
      initialDelayMs: 10,
      jitter: false,
      onRetry,
    });

    expect(result).toBe("RECOVERED");
    expect(fn).toHaveBeenCalledTimes(3);
    expect(onRetry).toHaveBeenCalledTimes(2);
  });

  it("throws when maxRetries is exceeded", async () => {
    const error = new Error("503 Service Unavailable");
    (error as any).status = 503;
    const fn = jest.fn().mockRejectedValue(error);

    await expect(
      withRpcRetry(fn, { maxRetries: 2, initialDelayMs: 5, jitter: false })
    ).rejects.toThrow("503 Service Unavailable");
    expect(fn).toHaveBeenCalledTimes(3); // 1 initial + 2 retries
  });

  it("fails immediately on non-retryable error without retrying", async () => {
    const fatalError = new Error("Invalid signature");
    const fn = jest.fn().mockRejectedValue(fatalError);

    await expect(withRpcRetry(fn, { maxRetries: 3 })).rejects.toThrow("Invalid signature");
    expect(fn).toHaveBeenCalledTimes(1);
  });
});
