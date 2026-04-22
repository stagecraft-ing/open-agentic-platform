// Stub for `~encore/auth` used by vitest when running outside the Encore
// runtime. The real module is generated into `encore.gen/auth` by the Encore
// CLI, which is git-ignored and absent in CI. Unit tests that import a
// module which in turn imports `getAuthData` just need the import to resolve;
// endpoint handlers that actually call it are exercised via `encore test`.

export interface AuthData {
  userId?: string;
  workspaceId?: string;
  [key: string]: unknown;
}

export function getAuthData(): AuthData | null {
  return null;
}
