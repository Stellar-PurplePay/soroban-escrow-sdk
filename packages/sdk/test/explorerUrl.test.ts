import {
  buildStellarExpertUrl,
  truncateIdentifier,
} from "../src/utils/explorerUrl";

describe("Explorer URL utilities", () => {
  const dummyTx = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
  const dummyAccount = "GDQP2KPQGKIHYJGXNUIYOMHARUARCA7DJT5FO2FFOOKY3IF53G4J7734";
  const dummyContract = "CA3D5KRYM6CB7OWQ6TWYRR3Z4T7ZQVNOOQ32GXZDF3G257JM5DOW63BP";

  it("returns empty string when identifier is empty", () => {
    expect(buildStellarExpertUrl("tx", "")).toBe("");
    expect(truncateIdentifier("")).toBe("");
  });

  it("builds testnet transaction explorer URL by default", () => {
    const url = buildStellarExpertUrl("tx", dummyTx);
    expect(url).toBe(`https://stellar.expert/explorer/testnet/tx/${dummyTx}`);
  });

  it("maps mainnet or public to public network segment", () => {
    const mainnetUrl = buildStellarExpertUrl("account", dummyAccount, { network: "mainnet" });
    expect(mainnetUrl).toBe(`https://stellar.expert/explorer/public/account/${dummyAccount}`);

    const publicUrl = buildStellarExpertUrl("account", dummyAccount, { network: "public" });
    expect(publicUrl).toBe(`https://stellar.expert/explorer/public/account/${dummyAccount}`);
  });

  it("supports futurenet and ledger numbers", () => {
    const futurenetUrl = buildStellarExpertUrl("contract", dummyContract, { network: "futurenet" });
    expect(futurenetUrl).toBe(`https://stellar.expert/explorer/futurenet/contract/${dummyContract}`);

    const ledgerUrl = buildStellarExpertUrl("ledger", 1234567, { network: "testnet" });
    expect(ledgerUrl).toBe("https://stellar.expert/explorer/testnet/ledger/1234567");
  });

  it("supports custom baseUrl without trailing slash issues", () => {
    const customUrl = buildStellarExpertUrl("tx", dummyTx, {
      baseUrl: "https://custom-explorer.stellar.org///",
      network: "testnet",
    });
    expect(customUrl).toBe(`https://custom-explorer.stellar.org/testnet/tx/${dummyTx}`);
  });

  it("truncates long identifiers correctly", () => {
    expect(truncateIdentifier(dummyTx)).toBe("e3b0c4...52b855");
    expect(truncateIdentifier(dummyAccount, 4, 4)).toBe("GDQP...7734");
    expect(truncateIdentifier("SHORT", 6, 6)).toBe("SHORT");
  });
});
