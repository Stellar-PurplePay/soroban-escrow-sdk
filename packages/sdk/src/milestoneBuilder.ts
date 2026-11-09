import { MilestoneInput } from "./types";
import { ValidationError } from "./errors";

/**
 * Utility to construct and balance milestone inputs satisfying Soroban contract invariants.
 */
export class MilestoneBuilder {
  /**
   * Generates N equally distributed milestones, automatically balancing remainder percentage onto the final milestone.
   *
   * @param count - Number of milestones (1 to 25)
   * @param titlePrefix - Optional description prefix (default "Milestone")
   * @returns Array of validated MilestoneInput objects summing to 100%
   */
  public static distributeEqualMilestones(
    count: number,
    titlePrefix = "Milestone"
  ): MilestoneInput[] {
    if (!Number.isInteger(count) || count < 1 || count > 25) {
      throw new ValidationError("Milestone count must be an integer between 1 and 25");
    }

    const basePct = Math.floor(100 / count);
    const remainder = 100 - basePct * count;

    const result: MilestoneInput[] = [];
    for (let i = 1; i <= count; i += 1) {
      const isLast = i === count;
      const percentage = isLast ? basePct + remainder : basePct;
      result.push({
        description: `${titlePrefix} ${i}`,
        percentage,
      });
    }

    return result;
  }

  /**
   * Generates percentage-balanced milestones from an array of arbitrary monetary amounts.
   *
   * @param items - Array of { description: string, amount: number | bigint }
   * @returns Array of MilestoneInput objects summing to 100%
   */
  public static fromAmounts(
    items: Array<{ description: string; amount: number | bigint }>
  ): MilestoneInput[] {
    if (!Array.isArray(items) || items.length === 0 || items.length > 25) {
      throw new ValidationError("Must provide between 1 and 25 milestone items");
    }

    const numericAmounts = items.map((item, idx) => {
      const val = Number(item.amount);
      if (isNaN(val) || val <= 0) {
        throw new ValidationError(`Milestone ${idx + 1}: Amount must be greater than zero`);
      }
      return val;
    });

    const total = numericAmounts.reduce((acc, curr) => acc + curr, 0);
    let allocatedPct = 0;
    const result: MilestoneInput[] = [];

    for (let i = 0; i < items.length; i += 1) {
      const isLast = i === items.length - 1;
      let pct = 0;
      if (isLast) {
        pct = 100 - allocatedPct;
      } else {
        pct = Math.round((numericAmounts[i] / total) * 100);
        if (pct < 1) pct = 1; // Contract requires percentage > 0
        allocatedPct += pct;
      }

      result.push({
        description: items[i].description,
        percentage: pct,
      });
    }

    // Safety check that total is exactly 100%
    const totalPct = result.reduce((acc, curr) => acc + curr.percentage, 0);
    if (totalPct !== 100) {
      result[result.length - 1].percentage += 100 - totalPct;
    }

    return result;
  }
}

