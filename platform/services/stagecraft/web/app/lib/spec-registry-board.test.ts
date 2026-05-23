// Spec 164 §2.2 / FR-003..FR-005 — lifecycle-state board projection
// unit tests. Pure derivation; no I/O.

import { describe, expect, test } from "vitest";
import type { SpecListRow } from "../../../api/specRegistry/types";
import {
  buildBoard,
  buildBoardWithGrouping,
  collectAmendedIds,
  placeSpec,
} from "./spec-registry-board";

function row(
  id: string,
  overrides: Partial<SpecListRow> = {},
): SpecListRow {
  return {
    id,
    title: `Spec ${id}`,
    status: "approved",
    implementation: "complete",
    kind: "platform",
    categories: [],
    summary: null,
    specPath: `specs/${id}/spec.md`,
    extraFrontmatter: {},
    relationshipFields: {},
    hasDecompositionOrigin: false,
    ...overrides,
  };
}

describe("placeSpec", () => {
  const noAmends = new Set<string>();

  test("draft → draft column", () => {
    const p = placeSpec(row("100-x", { status: "draft" }), noAmends);
    expect(p.column).toBe("draft");
    expect(p.lanes).toEqual([]);
  });

  test("approved + implementation:pending → impl-pending column", () => {
    const p = placeSpec(
      row("100-x", { status: "approved", implementation: "pending" }),
      noAmends,
    );
    expect(p.column).toBe("implementation-pending");
  });

  test("approved + implementation:in-progress → impl-in-progress column", () => {
    const p = placeSpec(
      row("100-x", { status: "approved", implementation: "in-progress" }),
      noAmends,
    );
    expect(p.column).toBe("implementation-in-progress");
  });

  test("approved + implementation:complete → impl-complete column", () => {
    const p = placeSpec(
      row("100-x", { status: "approved", implementation: "complete" }),
      noAmends,
    );
    expect(p.column).toBe("implementation-complete");
  });

  test("approved + implementation:n/a → approved column (visible fallback)", () => {
    const p = placeSpec(
      row("100-x", { status: "approved", implementation: "n/a" }),
      noAmends,
    );
    expect(p.column).toBe("approved");
  });

  test("approved + implementation:deferred → approved column (visible fallback)", () => {
    const p = placeSpec(
      row("100-x", { status: "approved", implementation: "deferred" }),
      noAmends,
    );
    expect(p.column).toBe("approved");
  });

  test("superseded → superseded lane only, no column", () => {
    const p = placeSpec(row("042", { status: "superseded" }), noAmends);
    expect(p.column).toBeNull();
    expect(p.lanes).toEqual(["superseded"]);
  });

  test("amended spec keeps its main column AND gains the amended lane", () => {
    const amended = new Set(["131-foo"]);
    const p = placeSpec(
      row("131-foo", { status: "approved", implementation: "complete" }),
      amended,
    );
    expect(p.column).toBe("implementation-complete");
    expect(p.lanes).toEqual(["amended"]);
  });

  test("amended + superseded coexist in lanes", () => {
    const amended = new Set(["042-old"]);
    const p = placeSpec(
      row("042-old", { status: "superseded" }),
      amended,
    );
    expect(p.column).toBeNull();
    expect(p.lanes).toEqual(["amended", "superseded"]);
  });
});

describe("collectAmendedIds", () => {
  test("walks bare-string amends edges and resolves to full ids", () => {
    const specs = [
      row("131-target", {}),
      row("200-amender", {
        relationshipFields: { amends: ["131"] },
      }),
    ];
    const result = collectAmendedIds(specs);
    expect([...result]).toEqual(["131-target"]);
  });

  test("walks typed amends edges with spec keys", () => {
    const specs = [
      row("131-target", {}),
      row("200-amender", {
        relationshipFields: {
          amends: [{ spec: "131", kind: "clarification" }],
        },
      }),
    ];
    expect([...collectAmendedIds(specs)]).toEqual(["131-target"]);
  });

  test("no amends edges → empty set", () => {
    expect(collectAmendedIds([row("100-x"), row("200-y")]).size).toBe(0);
  });

  test("amends edge pointing at an unknown id is silently ignored", () => {
    const specs = [
      row("200-amender", {
        relationshipFields: { amends: ["999"] },
      }),
    ];
    expect(collectAmendedIds(specs).size).toBe(0);
  });
});

describe("buildBoard (single-spec mode)", () => {
  test("distributes specs across all five columns and sorts by id", () => {
    const specs = [
      row("003-c", { status: "draft" }),
      row("001-a", { status: "draft" }),
      row("010-d", { status: "approved", implementation: "pending" }),
      row("020-e", { status: "approved", implementation: "in-progress" }),
      row("030-f", { status: "approved", implementation: "complete" }),
      row("040-g", { status: "approved", implementation: "n/a" }),
    ];
    const board = buildBoard(specs);
    expect(board.columns.draft.map((c) => c.id)).toEqual(["001-a", "003-c"]);
    expect(board.columns.approved.map((c) => c.id)).toEqual(["040-g"]);
    expect(board.columns["implementation-pending"].map((c) => c.id)).toEqual([
      "010-d",
    ]);
    expect(
      board.columns["implementation-in-progress"].map((c) => c.id),
    ).toEqual(["020-e"]);
    expect(board.columns["implementation-complete"].map((c) => c.id)).toEqual([
      "030-f",
    ]);
  });

  test("superseded specs land in the superseded lane only", () => {
    const specs = [
      row("042-old", { status: "superseded" }),
      row("100-live", { status: "approved", implementation: "complete" }),
    ];
    const board = buildBoard(specs);
    expect(board.lanes.superseded.map((c) => c.id)).toEqual(["042-old"]);
    expect(board.columns["implementation-complete"].map((c) => c.id)).toEqual([
      "100-live",
    ]);
    // No column lists the superseded spec.
    for (const col of Object.values(board.columns)) {
      expect(col.map((c) => c.id)).not.toContain("042-old");
    }
  });

  test("amended specs appear in their main column AND the amended lane", () => {
    const specs = [
      row("131-target", {
        status: "approved",
        implementation: "complete",
      }),
      row("200-amender", {
        status: "approved",
        implementation: "complete",
        relationshipFields: { amends: ["131"] },
      }),
    ];
    const board = buildBoard(specs);
    expect(
      board.columns["implementation-complete"].map((c) => c.id).sort(),
    ).toEqual(["131-target", "200-amender"]);
    expect(board.lanes.amended.map((c) => c.id)).toEqual(["131-target"]);
  });

  test("every card carries `kind: spec` in single-spec mode", () => {
    const specs = [row("001-a", { status: "draft" })];
    const board = buildBoard(specs);
    expect(board.columns.draft[0].kind).toBe("spec");
  });
});

describe("buildBoardWithGrouping (cluster cards)", () => {
  test("clusters by-category produce one card per group, placed by dominant column", () => {
    const specs = [
      row("001-a", {
        status: "approved",
        implementation: "complete",
        categories: ["auth"],
      }),
      row("002-b", {
        status: "approved",
        implementation: "complete",
        categories: ["auth"],
      }),
      row("003-c", {
        status: "approved",
        implementation: "pending",
        categories: ["auth"],
      }),
    ];
    const board = buildBoardWithGrouping(specs, "by-category");
    // auth cluster has two `complete` and one `pending` — dominant = complete (2 > 1)
    const completeCol = board.columns["implementation-complete"];
    expect(completeCol).toHaveLength(1);
    expect(completeCol[0].kind).toBe("cluster");
    expect(completeCol[0].label).toBe("auth");
    expect(completeCol[0].members.map((m) => m.id)).toEqual([
      "001-a",
      "002-b",
      "003-c",
    ]);
  });

  test("ties break toward the more-progressed column", () => {
    // Two members: one `approved`, one `complete`. Tie 1-1 ⇒ complete wins
    // because it has higher maturity.
    const specs = [
      row("001-a", {
        status: "approved",
        implementation: "n/a",
        categories: ["x"],
      }),
      row("002-b", {
        status: "approved",
        implementation: "complete",
        categories: ["x"],
      }),
    ];
    const board = buildBoardWithGrouping(specs, "by-category");
    expect(board.columns["implementation-complete"]).toHaveLength(1);
    expect(board.columns.approved).toHaveLength(0);
  });

  test("cluster with a superseded member appears in the superseded lane too", () => {
    const specs = [
      row("042-old", { status: "superseded", categories: ["x"] }),
      row("100-new", {
        status: "approved",
        implementation: "complete",
        categories: ["x"],
      }),
    ];
    const board = buildBoardWithGrouping(specs, "by-category");
    expect(board.lanes.superseded).toHaveLength(1);
    expect(board.lanes.superseded[0].kind).toBe("cluster");
    expect(board.columns["implementation-complete"]).toHaveLength(1);
  });

  test("cluster with all superseded members has no column placement", () => {
    const specs = [
      row("042-a", { status: "superseded", categories: ["x"] }),
      row("043-b", { status: "superseded", categories: ["x"] }),
    ];
    const board = buildBoardWithGrouping(specs, "by-category");
    for (const col of Object.values(board.columns)) {
      expect(col).toHaveLength(0);
    }
    expect(board.lanes.superseded).toHaveLength(1);
  });
});
