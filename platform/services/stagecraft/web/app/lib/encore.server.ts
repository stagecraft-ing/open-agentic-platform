import Client from "./client";

const DEFAULT_API_BASE = "http://localhost:4000";

export function createEncoreClient(request: Request): Client {
  // Explicit override wins. Used by the web-only dev loop where vite runs on
  // :3000 and the Encore backend runs on :4000 — without this, SSR loaders
  // would derive the base URL from request.url.origin and call themselves.
  const override = process.env.ENCORE_API_BASE_URL;
  if (override) return new Client(override);

  let baseUrl: string;
  try {
    const url = new URL(request.url);
    // Encore may use "origin" as internal host when forwarding; it doesn't resolve.
    baseUrl = url.hostname === "origin" ? DEFAULT_API_BASE : url.origin;
  } catch {
    baseUrl = DEFAULT_API_BASE;
  }
  return new Client(baseUrl);
}
