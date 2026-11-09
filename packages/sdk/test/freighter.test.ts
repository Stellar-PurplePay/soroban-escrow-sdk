import {
  signWithFreighter,
  FreighterWalletError,
  WalletConnectionStatus,
  FreighterApi,
} from "../src/freighter";
import { ValidationError } from "../src/errors";

describe("freighter signer helper", () => {
  const dummyXdr = "AAAAAGM8t...==";

  const createMockApi = (overrides?: Partial<FreighterApi>): FreighterApi => ({
    isConnected: jest.fn().mockResolvedValue(true),
    getUserInfo: jest.fn().mockResolvedValue({ publicKey: "GBTEST..." }),
    getNetworkDetails: jest.fn().mockResolvedValue({
      network: "TESTNET",
      networkPassphrase: "Test SDF Network ; September 2015",
    }),
    signTransaction: jest.fn().mockResolvedValue("SIGNED_XDR_RESULT"),
    ...overrides,
  });

  it("throws ValidationError when XDR is empty or not a string", async () => {
    await expect(signWithFreighter("")).rejects.toThrow(ValidationError);
    await expect(signWithFreighter(null as any)).rejects.toThrow(ValidationError);
  });

  it("throws FreighterWalletError when wallet is not installed", async () => {
    const originalWindow = global.window;
    delete (global as any).window;

    await expect(signWithFreighter(dummyXdr)).rejects.toThrow(FreighterWalletError);
    await expect(signWithFreighter(dummyXdr)).rejects.toMatchObject({
      code: WalletConnectionStatus.NotInstalled,
    });

    global.window = originalWindow;
  });

  it("throws when wallet is locked or not connected", async () => {
    const mockApi = createMockApi({
      isConnected: jest.fn().mockResolvedValue(false),
    });

    await expect(signWithFreighter(dummyXdr, undefined, mockApi)).rejects.toMatchObject({
      code: WalletConnectionStatus.Locked,
    });
  });

  it("detects network mismatch between Freighter and requested options", async () => {
    const mockApi = createMockApi({
      getNetworkDetails: jest.fn().mockResolvedValue({
        network: "PUBLIC",
        networkPassphrase: "Public Global Stellar Network ; September 2015",
      }),
    });

    await expect(
      signWithFreighter(
        dummyXdr,
        { networkPassphrase: "Test SDF Network ; September 2015" },
        mockApi
      )
    ).rejects.toMatchObject({
      code: WalletConnectionStatus.NetworkMismatch,
    });
  });

  it("signs successfully with valid Freighter response", async () => {
    const mockApi = createMockApi();

    const result = await signWithFreighter(
      dummyXdr,
      { networkPassphrase: "Test SDF Network ; September 2015" },
      mockApi
    );

    expect(result).toBe("SIGNED_XDR_RESULT");
    expect(mockApi.signTransaction).toHaveBeenCalledWith(dummyXdr, {
      networkPassphrase: "Test SDF Network ; September 2015",
      network: undefined,
      accountToSign: undefined,
    });
  });

  it("handles user rejection gracefully with USER_REJECTED code", async () => {
    const mockApi = createMockApi({
      signTransaction: jest.fn().mockRejectedValue(new Error("User declined transaction")),
    });

    await expect(signWithFreighter(dummyXdr, undefined, mockApi)).rejects.toMatchObject({
      code: "USER_REJECTED",
    });
  });
});
