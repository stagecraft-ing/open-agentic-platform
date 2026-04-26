import { describe, expect, test } from "vitest";
import {
  detectArtifactBodyKind,
  getArtifactDisplayBody,
  getArtifactMetadata,
  unwrapArtifactEnvelope,
  type FactoryArtifact,
} from "./artifact-body-viewer";

describe("detectArtifactBodyKind", () => {
  test("detects markdown by .md path", () => {
    expect(detectArtifactBodyKind({ path: "agents/orchestrator.md" })).toBe(
      "markdown"
    );
  });

  test("detects yaml by .schema.yaml path", () => {
    expect(
      detectArtifactBodyKind({ path: "Factory Agent/contracts/adapter-manifest.schema.yaml" })
    ).toBe("yaml");
  });

  test("detects yaml by plain .yml path", () => {
    expect(detectArtifactBodyKind({ path: "config.yml" })).toBe("yaml");
  });

  test("detects json by .json path", () => {
    expect(detectArtifactBodyKind({ path: "build-spec.json" })).toBe("json");
  });

  test("detects json by body starting with {", () => {
    expect(detectArtifactBodyKind({ body: '{ "kind": "manifest" }' })).toBe(
      "json"
    );
  });

  test("detects json by body starting with [ after whitespace", () => {
    expect(detectArtifactBodyKind({ body: "  [\n  1,\n  2\n]" })).toBe("json");
  });

  test("detects markdown when body starts with frontmatter fence", () => {
    expect(detectArtifactBodyKind({ body: "---\ntitle: hi\n---\n# Hello" })).toBe(
      "markdown"
    );
  });

  test("falls back to text", () => {
    expect(detectArtifactBodyKind({ body: "just some plain text" })).toBe("text");
  });

  test("falls back to text when body is missing and path is unknown", () => {
    expect(detectArtifactBodyKind({})).toBe("text");
  });

  test("path extension wins over body sniff", () => {
    expect(
      detectArtifactBodyKind({ path: "thing.md", body: '{ "x": 1 }' })
    ).toBe("markdown");
  });

  test("object body (jsonb manifest) is detected as json", () => {
    expect(
      detectArtifactBodyKind({
        body: { entry: "x", skills: { a: { path: "p", body: "y" } } },
      })
    ).toBe("json");
  });

  test("array body is detected as json", () => {
    expect(detectArtifactBodyKind({ body: [1, 2, 3] })).toBe("json");
  });
});

describe("getArtifactDisplayBody", () => {
  test("returns raw string body unchanged", () => {
    const raw = "line 1\nline 2\nline 3";
    expect(getArtifactDisplayBody({ body: raw })).toBe(raw);
  });

  test("stringifies object bodies as pretty JSON", () => {
    const result = getArtifactDisplayBody({ body: { a: 1, b: [2, 3] } });
    expect(result).toBe('{\n  "a": 1,\n  "b": [\n    2,\n    3\n  ]\n}');
  });

  test("returns empty string for null body", () => {
    expect(getArtifactDisplayBody({ body: null })).toBe("");
  });

  test("returns empty string for undefined body", () => {
    expect(getArtifactDisplayBody({})).toBe("");
  });

  test("does not double-stringify already-string body", () => {
    const raw = '{"already":"stringified"}';
    expect(getArtifactDisplayBody({ body: raw })).toBe(raw);
  });
});

describe("getArtifactMetadata", () => {
  test("excludes the body field", () => {
    const artifact: FactoryArtifact = {
      name: "adapter-manifest",
      version: "abcdef012345",
      sourceSha: "deadbeef",
      syncedAt: "2026-04-25T17:36:12.000Z",
      body: "huge yaml string\n…\n",
    };
    const metadata = getArtifactMetadata(artifact);
    expect(metadata).not.toHaveProperty("body");
    expect(metadata).toMatchObject({
      name: "adapter-manifest",
      version: "abcdef012345",
      sourceSha: "deadbeef",
      syncedAt: "2026-04-25T17:36:12.000Z",
    });
  });

  test("preserves arbitrary extra fields", () => {
    const metadata = getArtifactMetadata({
      name: "x",
      body: "y",
      sha: "1234",
      path: "schemas/x.schema.yaml",
    });
    expect(metadata).toEqual({
      name: "x",
      sha: "1234",
      path: "schemas/x.schema.yaml",
    });
  });
});

describe("unwrapArtifactEnvelope", () => {
  test("hoists path + body from {path, body} envelope", () => {
    const unwrapped = unwrapArtifactEnvelope({
      name: "adapter-manifest",
      body: { path: "schemas/adapter-manifest.schema.yaml", body: "key: value\n" },
    });
    expect(unwrapped.path).toBe("schemas/adapter-manifest.schema.yaml");
    expect(unwrapped.body).toBe("key: value\n");
  });

  test("does not overwrite an existing top-level path", () => {
    const unwrapped = unwrapArtifactEnvelope({
      path: "outer/path",
      body: { path: "inner/path", body: "x" },
    });
    expect(unwrapped.path).toBe("outer/path");
    expect(unwrapped.body).toBe("x");
  });

  test("leaves non-envelope bodies alone", () => {
    const original: FactoryArtifact = {
      name: "manifest",
      body: { entry: "x", skills: {} },
    };
    expect(unwrapArtifactEnvelope(original)).toEqual(original);
  });

  test("leaves string bodies alone", () => {
    const original: FactoryArtifact = { name: "x", body: "hello" };
    expect(unwrapArtifactEnvelope(original)).toEqual(original);
  });
});

describe("end-to-end body preservation", () => {
  test("a contract envelope carrying YAML round-trips with real newlines", () => {
    const artifact = unwrapArtifactEnvelope({
      name: "adapter-manifest",
      body: {
        path: "Factory Agent/contracts/adapter-manifest.schema.yaml",
        body: "$schema: https://json-schema.org/draft/2020-12/schema\ntitle: AdapterManifest\n",
      },
    });
    expect(detectArtifactBodyKind(artifact)).toBe("yaml");
    expect(getArtifactDisplayBody(artifact)).toContain("\n");
    expect(getArtifactDisplayBody(artifact)).not.toContain("\\n");
  });
});
