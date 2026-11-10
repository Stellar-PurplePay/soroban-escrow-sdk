import * as fs from "fs";
import * as path from "path";
// @ts-ignore
import * as yaml from "js-yaml";

describe("SDK CI Workflow Validator", () => {
  const ciPath = path.resolve(__dirname, "../../../.github/workflows/ci.yml");

  it("should parse .github/workflows/ci.yml as valid YAML", () => {
    expect(fs.existsSync(ciPath)).toBe(true);
    const content = fs.readFileSync(ciPath, "utf8");
    const parsed: any = yaml.load(content);
    expect(parsed).toBeDefined();
    expect(parsed.name).toBe("CI");
  });

  it("should configure wasm32v1-none compilation for smart contract job", () => {
    const content = fs.readFileSync(ciPath, "utf8");
    const parsed: any = yaml.load(content);
    const contractJob = parsed.jobs.contract;
    expect(contractJob).toBeDefined();

    const toolchainStep = contractJob.steps.find((s: any) =>
      s.uses?.includes("dtolnay/rust-toolchain")
    );
    expect(toolchainStep).toBeDefined();
    expect(toolchainStep.with?.targets).toContain("wasm32v1-none");
  });

  it("should configure Node matrix testing and dependency audit for SDK job", () => {
    const content = fs.readFileSync(ciPath, "utf8");
    const parsed: any = yaml.load(content);
    const sdkJob = parsed.jobs.sdk;
    expect(sdkJob).toBeDefined();
    expect(sdkJob.strategy?.matrix?.["node-version"]).toEqual(
      expect.arrayContaining([18, 20])
    );

    const auditStep = sdkJob.steps.find((s: any) =>
      s.name?.includes("Dependency Audit")
    );
    expect(auditStep).toBeDefined();
  });
});
