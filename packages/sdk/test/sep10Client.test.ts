import { Sep10Client } from "../src/sep10Client";
import { NetworkError, ValidationError } from "../src/errors";

describe("Sep10Client", () => {
  const account = "GCLIENT1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234";
  const fakeChallengeXdr = "AAAA...fake...challenge...xdr";
  const fakeSignedXdr = "AAAA...fake...signed...xdr";
  const fakeToken =
    "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJHQ0xJRU5UMTIzNCIsImlzcyI6InB1cnBsZXBheS5hcHAiLCJleHAiOjE5OTk5OTk5OTl9.signature";

  it("requires authEndpoint in configuration", () => {
    expect(() => new Sep10Client({ authEndpoint: "" })).toThrow(ValidationError);
  });

  it("fetches challenge transaction successfully", async () => {
    const mockFetch = jest.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ transaction: fakeChallengeXdr }),
    });

    const client = new Sep10Client({
      authEndpoint: "https://api.purplepay.app/api/auth/sep10",
      fetchFn: mockFetch as any,
    });

    const challenge = await client.getChallenge(account);
    expect(challenge.transaction).toBe(fakeChallengeXdr);
    expect(mockFetch).toHaveBeenCalledWith(
      `https://api.purplepay.app/api/auth/sep10/challenge?account=${account}`,
      expect.objectContaining({ method: "GET" })
    );
  });

  it("exchanges signed challenge transaction for session token", async () => {
    const mockFetch = jest.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ token: fakeToken, account }),
    });

    const client = new Sep10Client({
      authEndpoint: "https://api.purplepay.app/api/auth/sep10",
      fetchFn: mockFetch as any,
    });

    const res = await client.exchangeToken(fakeSignedXdr);
    expect(res.token).toBe(fakeToken);
    expect(res.account).toBe(account);
    expect(mockFetch).toHaveBeenCalledWith(
      "https://api.purplepay.app/api/auth/sep10/token",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ transaction: fakeSignedXdr }),
      })
    );
  });

  it("authenticates end-to-end with sign callback", async () => {
    const mockFetch = jest
      .fn()
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ transaction: fakeChallengeXdr }),
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ token: fakeToken, account }),
      });

    const signCallback = jest.fn().mockResolvedValue(fakeSignedXdr);

    const client = new Sep10Client({
      authEndpoint: "https://api.purplepay.app/api/auth/sep10",
      fetchFn: mockFetch as any,
    });

    const result = await client.authenticate(account, signCallback);
    expect(result.token).toBe(fakeToken);
    expect(signCallback).toHaveBeenCalledWith(fakeChallengeXdr);
  });

  it("handles HTTP error during challenge fetch", async () => {
    const mockFetch = jest.fn().mockResolvedValue({
      ok: false,
      status: 400,
      text: async () => "Account not found",
    });

    const client = new Sep10Client({
      authEndpoint: "https://api.purplepay.app/api/auth/sep10",
      fetchFn: mockFetch as any,
    });

    await expect(client.getChallenge(account)).rejects.toThrow(NetworkError);
  });

  it("parses JWT claims without external dependencies", () => {
    const claims = Sep10Client.parseJwtClaims(fakeToken);
    expect(claims.sub).toBe("GCLIENT1234");
    expect(claims.iss).toBe("purplepay.app");
    expect(claims.exp).toBe(1999999999);
  });

  it("throws ValidationError when parsing invalid JWT", () => {
    expect(() => Sep10Client.parseJwtClaims("invalid")).toThrow(ValidationError);
    expect(() => Sep10Client.parseJwtClaims("")).toThrow(ValidationError);
  });
});
