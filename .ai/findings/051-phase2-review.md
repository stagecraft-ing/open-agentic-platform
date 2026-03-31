# 051 Phase 2 Review — Concurrency + Queue Control

**Reviewer:** claude
**Date:** 2026-03-30
**Verdict:** Phase 2 APPROVED — no blockers for Phase 3.

## Requirement coverage

### FR-002 — Configurable max concurrent agent count, FIFO queuing

**Status: SATISFIED**

- `FifoConcurrencyLimiter` constructor accepts `maxConcurrent` (default 4) — matches spec default.
- `acquire()` returns immediately when `activeCount < maxConcurrent`, otherwise enqueues a `QueueEntry` in FIFO order via `queue.push()`.
- `releaseOne()` dequeues via `queue.shift()` — strict FIFO.
- Metrics expose `maxConcurrent`, `activeCount`, `queuedCount` for API layer consumption (plan deliverable: "Queue/active status metrics exposed to API layer").

### SC-002 — Third agent queues when max=2

**Status: SATISFIED**

- Test at `concurrency.test.ts:5–43` directly validates: `max=2`, acquire A + B succeed, third acquire remains pending (`queuedCount: 1`), `thirdResolved === false` after microtask flush, resolves only after `releaseA()`.
- Post-release metrics confirm correct state transitions (`activeCount: 2 → 2`, `queuedCount: 1 → 0` after dequeue; `activeCount: 0` after all releases).

## Implementation analysis

### Correctness

1. **Double-release guard** (`concurrency.ts:50–55`): `released` boolean prevents double-decrement of `activeCount`. Sound — each `createRelease()` closure captures its own `released` flag.

2. **Release-then-dequeue atomicity** (`releaseOne()` at lines 58–70): Decrements `activeCount` first, then immediately checks queue. If a pending entry exists, increments `activeCount` and resolves the pending promise. This is safe in single-threaded JS — no interleaving between decrement and re-increment.

3. **Defensive decrement** (`if (this.activeCount > 0)` at line 59): Protects against underflow if `releaseOne` is somehow called with `activeCount === 0`. This cannot happen with correct usage due to the double-release guard, but the defensive check is harmless.

4. **FIFO guarantee**: `push()` + `shift()` on a plain array is correct FIFO. The FIFO order test (`concurrency.test.ts:45–84`) validates with `max=1` and 3 queued entries — wake-up order is `second → third → fourth`.

### Design

- **Promise-based semaphore** pattern is clean and idiomatic for async TypeScript. The caller receives a `release` function — no need for a separate `release(token)` API.
- **No external dependencies** — pure TypeScript, no runtime deps.
- **Subpath export** (`./concurrency`) allows targeted import without pulling in worktree-manager and its `node:*` deps.
- **Type export** (`ConcurrencyMetrics`) separated from class export in `index.ts` — correct.

## Findings

| ID | Severity | Description |
|----|----------|-------------|
| P2-001 | LOW | No test for double-release safety — calling `release()` twice should be a no-op. The guard exists (`released` flag) but is untested. |
| P2-002 | LOW | No test for `maxConcurrent` default value (4). Tests use explicit values (1, 2). A one-liner `expect(new FifoConcurrencyLimiter().getMetrics().maxConcurrent).toBe(4)` would confirm spec default. |
| P2-003 | LOW | No cancellation API for queued acquires. If a caller wants to abandon a queued request (e.g., user cancels spawn before slot opens), there's no way to remove it from the queue. Phase 3 (timeout) may surface this need. |
| P2-004 | INFO | `queue` is a plain `Array` — `shift()` is O(n). For realistic concurrency limits (4–16 agents), this is negligible. Would only matter at hundreds of queued entries. |
| P2-005 | INFO | No `destroy()` / `drain()` method to reject all pending acquires on shutdown. Phase 3 runner lifecycle may need this — flag for Phase 3 design consideration. |
| P2-006 | INFO | Non-integer floats like `3.7` are caught by the `Number.isInteger()` guard. `NaN` and `Infinity` are also caught. Negative values caught by `<= 0`. Input validation is thorough. |

## Phase 1 prior findings status

- **P1-001** (`sanitizeSegment` collision potential) — remains LOW, no change expected in Phase 2.
- **P1-002** (no `startPoint` test) — remains LOW.
- **P1-003** (no direct porcelain parser test) — remains LOW.

## Verdict

Phase 2 implementation is clean, correct, and spec-faithful. FR-002 and SC-002 are fully satisfied. The FIFO semaphore pattern is well-suited for Phase 3 integration (agent runner will call `acquire()` before spawn, `release()` on terminal state). P2-003 (cancellation) and P2-005 (drain) are design considerations for Phase 3 but not blockers.

**No blockers for Phase 3** (agent runner lifecycle + timeout).
