// Lightweight mock for encore.dev/api used by vitest when running outside
// the Encore runtime (e.g. CI with `npm test`). The `api()` wrapper simply
// returns the handler function so unit tests can call endpoints directly.

export function api(_options: any, handler: any) {
  return handler;
}

export class APIError extends Error {
  public readonly code: string;
  constructor(code: string, message: string) {
    super(message);
    this.code = code;
  }
}
