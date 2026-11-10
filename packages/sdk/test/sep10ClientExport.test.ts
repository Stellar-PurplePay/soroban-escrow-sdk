import * as Sdk from "../src/index";

describe("sep10Client barrel exports", () => {
  it("exports Sep10Client class", () => {
    expect(typeof Sdk.Sep10Client).toBe("function");
    expect(typeof Sdk.Sep10Client.parseJwtClaims).toBe("function");
  });

  it("exports NetworkError class", () => {
    expect(typeof Sdk.NetworkError).toBe("function");
    const err = new Sdk.NetworkError("connection refused", 503);
    expect(err.code).toBe("NETWORK_ERROR");
    expect(err.status).toBe(503);
  });
});
