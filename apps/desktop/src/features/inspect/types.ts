/**
 * Feature 032 — T003: typed inspect shell flow (xray scan path).
 * Not coupled to git/governance; those are later tasks.
 */

/** Normalized outcome of an `xray_scan_project` invoke + payload classification. */
export type InspectFlowState =
  | { status: 'idle' }
  | { status: 'loading' }
  | { status: 'success'; payload: unknown }
  | { status: 'error'; message: string }
  | { status: 'degraded'; payload: unknown; reason: string };
