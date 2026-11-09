import { MilestoneBuilder } from "../src/milestoneBuilder";
import { ValidationError } from "../src/errors";

describe("MilestoneBuilder", () => {
  describe("distributeEqualMilestones", () => {
    it("distributes 3 milestones summing to exactly 100%", () => {
      const milestones = MilestoneBuilder.distributeEqualMilestones(3);
      expect(milestones).toHaveLength(3);
      expect(milestones[0].percentage).toBe(33);
      expect(milestones[1].percentage).toBe(33);
      expect(milestones[2].percentage).toBe(34); // remainder applied to last

      const sum = milestones.reduce((acc, m) => acc + m.percentage, 0);
      expect(sum).toBe(100);
    });

    it("distributes single milestone as 100%", () => {
      const milestones = MilestoneBuilder.distributeEqualMilestones(1, "Phase");
      expect(milestones).toHaveLength(1);
      expect(milestones[0].percentage).toBe(100);
      expect(milestones[0].description).toBe("Phase 1");
    });

    it("rejects invalid count (<1 or >25)", () => {
      expect(() => MilestoneBuilder.distributeEqualMilestones(0)).toThrow(ValidationError);
      expect(() => MilestoneBuilder.distributeEqualMilestones(26)).toThrow(ValidationError);
    });
  });

  describe("fromAmounts", () => {
    it("computes integer percentages from arbitrary amounts summing to 100%", () => {
      const items = [
        { description: "Design", amount: 250 },
        { description: "Development", amount: 500 },
        { description: "QA & Deployment", amount: 250 },
      ];

      const milestones = MilestoneBuilder.fromAmounts(items);
      expect(milestones).toHaveLength(3);
      expect(milestones[0].percentage).toBe(25);
      expect(milestones[1].percentage).toBe(50);
      expect(milestones[2].percentage).toBe(25);

      const sum = milestones.reduce((acc, m) => acc + m.percentage, 0);
      expect(sum).toBe(100);
    });

    it("ensures rounding always balances to 100% on uneven amounts", () => {
      const items = [
        { description: "Task 1", amount: 33 },
        { description: "Task 2", amount: 33 },
        { description: "Task 3", amount: 34 },
      ];

      const milestones = MilestoneBuilder.fromAmounts(items);
      const sum = milestones.reduce((acc, m) => acc + m.percentage, 0);
      expect(sum).toBe(100);
    });

    it("rejects non-positive amount", () => {
      expect(() =>
        MilestoneBuilder.fromAmounts([{ description: "Bad", amount: 0 }])
      ).toThrow(ValidationError);
    });

    it("verifies equal milestone distribution strictly sums to 100% across all supported counts (1..25)", () => {
      for (let count = 1; count <= 25; count++) {
        const milestones = MilestoneBuilder.distributeEqualMilestones(count);
        expect(milestones).toHaveLength(count);

        const sum = milestones.reduce((acc, m) => acc + m.percentage, 0);
        expect(sum).toBe(100);

        for (const m of milestones) {
          expect(m.percentage).toBeGreaterThan(0);
          expect(m.percentage).toBeLessThanOrEqual(100);
        }
      }
    });

    it("verifies fromAmounts property conservation across randomized amount distributions", () => {
      for (let trial = 0; trial < 100; trial++) {
        const count = 2 + Math.floor(Math.random() * 10); // 2 to 11 items
        const items = Array.from({ length: count }, (_, idx) => ({
          description: `Task ${idx + 1}`,
          amount: 1 + Math.floor(Math.random() * 100_000),
        }));

        const milestones = MilestoneBuilder.fromAmounts(items);
        expect(milestones).toHaveLength(count);

        const sum = milestones.reduce((acc, m) => acc + m.percentage, 0);
        expect(sum).toBe(100);
      }
    });
  });
});

