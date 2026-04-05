# Test Pattern

Encore tests use Vitest and call endpoint functions directly as regular async
functions. Encore sets up the test environment (database, pub/sub, services)
automatically -- no mocking of infrastructure required.

## Convention

- Test files sit next to the code they test: `{name}.test.ts`
- Import `describe`, `test`, `expect` from `vitest`
- Import the endpoint function directly for unit-style tests
- Import from `~encore/clients` for integration tests that cross services
- Use `test.each` for data-driven tests with multiple input/expected pairs
- Encore provisions a real test database -- no need to mock SQL

## Template

### Unit test (single endpoint)

```ts
import { describe, expect, test } from "vitest";
import { {endpoint} } from "./{file}";

describe("{endpoint}", () => {
  test("{testDescription}", async () => {
    const resp = await {endpoint}({params});
    expect(resp.{field}).toBe({expected});
  });
});
```

### Data-driven test

```ts
import { describe, expect, test } from "vitest";
import { {endpoint} } from "./{file}";

describe("{endpoint}", () => {
  test.each([
    { input: {value1}, expected: {result1} },
    { input: {value2}, expected: {result2} },
  ])(
    "{testLabel}",
    async ({ input, expected }) => {
      const resp = await {endpoint}({inputMapping});
      expect(resp.{field}).toBe(expected);
    },
  );
});
```

### Integration test (cross-service)

```ts
import { describe, expect, test } from "vitest";
import { {endpoint} } from "./{file}";
import { {remoteService} } from "~encore/clients";

describe("{endpoint}", () => {
  test("{testDescription}", async () => {
    const created = await {remoteService}.{createEndpoint}({setupParams});
    const resp = await {endpoint}({paramsUsingCreated});
    expect(resp.{field}).toBe({expected});
  });
});
```

## Example

Data-driven endpoint test -- `api/monitor/ping.test.ts`:

```ts
import { describe, expect, test } from "vitest";
import { ping } from "./ping";

describe("ping", () => {
  test.each([
    { site: "google.com", expected: true },
    { site: "https://encore.dev", expected: true },
    { site: "https://not-a-real-site.xyz", expected: false },
    { site: "invalid://scheme", expected: false },
  ])(
    `should verify that $site is ${"$expected" ? "up" : "down"}`,
    async ({ site, expected }) => {
      const resp = await ping({ url: site });
      expect(resp.up).toBe(expected);
    },
  );
});
```

Cross-service integration test -- `api/monitor/check.test.ts`:

```ts
import { expect, describe, test } from "vitest";
import { check } from "./check";
import { site } from "~encore/clients";

describe("check", () => {
  test("it should add a site and check if it's up", async () => {
    const url = `encore.dev?${Math.random().toString(36).substring(7)}`;
    const obj = await site.add({ url });     // calls site service
    const resp = await check({ siteID: obj.id });  // calls local endpoint
    expect(resp.up).toBe(true);
  });
});
```

## Rules

1. Test files must be named `*.test.ts` and live beside the source file.
2. Import endpoints as plain functions -- call them directly, not via HTTP.
3. Use `~encore/clients` when the test needs to call a different service to
   set up data. Encore wires the services in the test environment.
4. `test.each` is preferred when validating multiple inputs against the same
   assertion logic. Use `$varName` in the test name for interpolation.
5. Encore provisions a real database per test run -- assert against actual state.
6. No `beforeAll`/`afterAll` needed for infra. Encore handles setup automatically.
7. Keep tests fast: one endpoint call per test (or small setup + call sequence).
8. Run tests with `encore test` -- this starts the Encore test runtime.
