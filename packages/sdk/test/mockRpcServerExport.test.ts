import { MockSorobanRpcServer } from "../src/index";

describe("MockSorobanRpcServer Barrel Export", () => {
  it("should successfully import MockSorobanRpcServer from SDK index", () => {
    expect(MockSorobanRpcServer).toBeDefined();
    const server = new MockSorobanRpcServer();
    expect(typeof server.handleRequest).toBe("function");
    expect(typeof server.advanceLedger).toBe("function");
  });
});
