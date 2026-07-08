// Spec 227: TTL semantics of the per-org module-catalog cache.

import { afterEach, describe, expect, test } from "vitest";
import {
  MODULE_CATALOG_CACHE_TTL_MS,
  clearModuleCatalogCache,
  getCachedModuleCatalog,
} from "./moduleCatalogCache";
import type { ModuleDescriptor } from "../projects/scaffold/moduleCatalog";

afterEach(() => clearModuleCatalogCache());

function descriptor(id: string): ModuleDescriptor {
  return {
    id,
    displayName: id,
    category: "Data",
    description: "",
    requires: [],
    conflicts: [],
  };
}

describe("getCachedModuleCatalog", () => {
  test("loads once within the TTL window", async () => {
    let calls = 0;
    const load = async () => {
      calls += 1;
      return [descriptor("data-redis")];
    };
    let clock = 1_000;
    const now = () => clock;

    const first = await getCachedModuleCatalog("org-1", load, now);
    clock += MODULE_CATALOG_CACHE_TTL_MS - 1;
    const second = await getCachedModuleCatalog("org-1", load, now);

    expect(calls).toBe(1);
    expect(second).toBe(first);
  });

  test("reloads once the TTL has elapsed", async () => {
    let calls = 0;
    const load = async () => {
      calls += 1;
      return [descriptor("data-redis")];
    };
    let clock = 1_000;
    const now = () => clock;

    await getCachedModuleCatalog("org-1", load, now);
    clock += MODULE_CATALOG_CACHE_TTL_MS;
    await getCachedModuleCatalog("org-1", load, now);

    expect(calls).toBe(2);
  });

  test("caches per org id", async () => {
    let calls = 0;
    const load = async () => {
      calls += 1;
      return [descriptor("x")];
    };
    const now = () => 5_000;

    await getCachedModuleCatalog("org-1", load, now);
    await getCachedModuleCatalog("org-2", load, now);

    expect(calls).toBe(2);
  });

  test("clearModuleCatalogCache forces a reload", async () => {
    let calls = 0;
    const load = async () => {
      calls += 1;
      return [descriptor("x")];
    };
    const now = () => 5_000;

    await getCachedModuleCatalog("org-1", load, now);
    clearModuleCatalogCache();
    await getCachedModuleCatalog("org-1", load, now);

    expect(calls).toBe(2);
  });
});
