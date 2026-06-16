import { describe, expect, it } from "vitest";
import { listShapes, selectChart } from "./chartSelector";

describe("chartSelector (spec 136 Phase 2)", () => {
  it("resolves tenant-hello to the matching Helm chart", () => {
    const sel = selectChart({ shape: "tenant-hello" });
    expect(sel.chart).toBe("tenant-hello");
    expect(sel.version).toBe("0.1.0");
  });

  it("resolves aim-vue-encore (the factory reference shape, spec 214)", () => {
    const sel = selectChart({ shape: "aim-vue-encore" });
    expect(sel.chart).toBe("aim-vue-encore");
    expect(sel.version).toBe("0.1.0");
  });

  it("throws on an unregistered shape rather than falling back silently", () => {
    expect(() =>
      // Deliberately violate the type to exercise the runtime guard —
      // unknown shapes must fail at deploy time, not silently default.
      selectChart({ shape: "unknown-shape" as unknown as "tenant-hello" }),
    ).toThrow(/no chart registered for shape/);
  });

  it("listShapes enumerates every registered shape", () => {
    const shapes = listShapes();
    expect(shapes).toEqual(["tenant-hello", "aim-vue-encore"]);
  });
});
